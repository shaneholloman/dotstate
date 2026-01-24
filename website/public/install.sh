#!/usr/bin/env bash
# DotState Installation Script (inspired by opencode's excellent installer)
# Downloads pre-built binary, falls back to Cargo if needed

set -uo pipefail

REPO="serkanyersen/dotstate"
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="dotstate"

# Colors
DIM='\033[2m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

# Options
requested_version="${DOTSTATE_VERSION:-latest}"
no_modify_path=false
force_install=false

usage() {
    cat <<EOF
DotState Installer

Usage: install.sh [options]

Options:
    -h, --help              Show this help message
    -v, --version <version> Install specific version (e.g., v0.2.16)
    -f, --force             Reinstall even if same version exists
        --no-modify-path    Don't modify shell config files

Examples:
    curl -fsSL https://dotstate.serkan.dev/install.sh | bash
    curl -fsSL https://dotstate.serkan.dev/install.sh | bash -s -- -v v0.2.16
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            usage
            exit 0
            ;;
        -v|--version)
            if [[ -n "${2:-}" ]]; then
                requested_version="$2"
                shift 2
            else
                echo -e "${RED}Error: --version requires a version argument${NC}"
                exit 1
            fi
            ;;
        -f|--force)
            force_install=true
            shift
            ;;
        --no-modify-path)
            no_modify_path=true
            shift
            ;;
        *)
            echo -e "${YELLOW}Unknown option: $1${NC}" >&2
            shift
            ;;
    esac
done

print_msg() {
    local level=$1
    local msg=$2
    case $level in
        info)    echo -e "${DIM}${msg}${NC}" ;;
        success) echo -e "${GREEN}${msg}${NC}" ;;
        warn)    echo -e "${YELLOW}${msg}${NC}" ;;
        error)   echo -e "${RED}${msg}${NC}" ;;
        plain)   echo -e "${msg}" ;;
    esac
}

draw_progress() {
    local percent=$1
    local width=30
    [[ $percent -gt 100 ]] && percent=100
    local filled=$((percent * width / 100))
    local empty=$((width - filled))

    local bar=$(printf "%${filled}s" | tr ' ' '━')
    local space=$(printf "%${empty}s" | tr ' ' '─')

    printf "\r  ${CYAN}%s%s${NC} %3d%%" "$bar" "$space" "$percent" >&2
}

download_with_progress() {
    local url=$1
    local output=$2

    # Non-TTY: simple download
    if [[ ! -t 2 ]]; then
        curl -fsSL "$url" -o "$output"
        return $?
    fi

    # Get file size (follow redirects)
    local total=$(curl -sIL "$url" | grep -i content-length | tail -1 | awk '{print $2}' | tr -d '\r')

    if [[ -z "$total" || "$total" -lt 1000 ]]; then
        # Can't get size, use simple download with curl's bar
        curl -fsSL "$url" -o "$output"
        return $?
    fi

    # Hide cursor
    printf '\033[?25l' >&2
    echo ""
    draw_progress 0

    # Start download in background
    curl -fsSL "$url" -o "$output" --limit-rate 0 &
    local pid=$!

    # Small delay to let file be created
    sleep 0.05

    # Monitor progress
    while kill -0 $pid 2>/dev/null; do
        if [[ -f "$output" ]]; then
            local current=$(stat -f%z "$output" 2>/dev/null || stat -c%s "$output" 2>/dev/null || echo 0)
            if [[ $current -gt 0 ]]; then
                local percent=$((current * 100 / total))
                draw_progress $percent
            fi
        fi
        sleep 0.05
    done

    wait $pid
    local status=$?

    # Final update
    draw_progress 100
    echo "" >&2

    # Show cursor
    printf '\033[?25h' >&2

    return $status
}

detect_system() {
    case "$(uname -s)" in
        Linux*)  OS="linux" ;;
        Darwin*) OS="macos" ;;
        MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
        *)
            print_msg error "Unsupported OS: $(uname -s)"
            exit 1
            ;;
    esac

    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64|amd64) ARCH="x86_64" ;;
        arm64|aarch64) ARCH="arm64" ;;
        *)
            print_msg error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    # Detect Rosetta on macOS (x64 process on arm64 hardware)
    if [[ "$OS" == "macos" && "$ARCH" == "x86_64" ]]; then
        if sysctl -n sysctl.proc_translated 2>/dev/null | grep -q 1; then
            ARCH="arm64"
        fi
    fi
}

get_asset_name() {
    if [[ "$OS" == "linux" ]]; then
        if [[ "$ARCH" == "arm64" ]]; then
            echo "dotstate-aarch64-unknown-linux-musl.tar.gz"
        else
            echo "dotstate-x86_64-unknown-linux-musl.tar.gz"
        fi
    elif [[ "$OS" == "macos" ]]; then
        if [[ "$ARCH" == "arm64" ]]; then
            echo "dotstate-aarch64-apple-darwin.tar.gz"
        else
            echo "dotstate-x86_64-apple-darwin.tar.gz"
        fi
    elif [[ "$OS" == "windows" ]]; then
        echo "dotstate-x86_64-pc-windows-msvc.exe.tar.gz"
    fi
}

check_existing_version() {
    if command -v dotstate &>/dev/null; then
        local installed=$(dotstate --version 2>/dev/null | head -1 | awk '{print $2}')
        if [[ -n "$installed" ]]; then
            echo "$installed"
            return 0
        fi
    fi
    echo ""
}

fetch_latest_version() {
    curl -s "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
}

configure_path() {
    [[ "$no_modify_path" == "true" ]] && return

    # Already in PATH?
    if echo "$PATH" | grep -q "$INSTALL_DIR"; then
        return
    fi

    local shell_name=$(basename "$SHELL")
    local config_file=""
    local path_cmd=""

    case "$shell_name" in
        zsh)
            config_file="${ZDOTDIR:-$HOME}/.zshrc"
            path_cmd="export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
        bash)
            config_file="$HOME/.bashrc"
            [[ ! -f "$config_file" ]] && config_file="$HOME/.bash_profile"
            path_cmd="export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
        fish)
            config_file="$HOME/.config/fish/config.fish"
            path_cmd="fish_add_path $INSTALL_DIR"
            ;;
        *)
            config_file="$HOME/.profile"
            path_cmd="export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
    esac

    if [[ -f "$config_file" ]]; then
        if ! grep -Fq "$INSTALL_DIR" "$config_file" 2>/dev/null; then
            echo "" >> "$config_file"
            echo "# dotstate" >> "$config_file"
            echo "$path_cmd" >> "$config_file"
            print_msg info "Added to PATH in $config_file"
            print_msg info "Restart your shell or run: source $config_file"
        fi
    else
        print_msg warn "Add to your shell config: $path_cmd"
    fi

    # GitHub Actions support
    if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
        echo "$INSTALL_DIR" >> "$GITHUB_PATH"
    fi
}

install_via_cargo() {
    if ! command -v cargo &>/dev/null; then
        return 1
    fi

    print_msg info "Trying Cargo installation..."
    if cargo install dotstate 2>/dev/null; then
        return 0
    fi
    return 1
}

download_binary() {
    detect_system

    local asset_name=$(get_asset_name)
    local version_display="$requested_version"

    # Resolve version
    if [[ "$requested_version" == "latest" ]]; then
        version_display=$(fetch_latest_version)
        if [[ -z "$version_display" ]]; then
            print_msg error "Failed to fetch latest version"
            return 1
        fi
    fi

    # Strip 'v' prefix for comparison
    local version_num="${version_display#v}"

    # Check if already installed
    local existing=$(check_existing_version)
    if [[ -n "$existing" && "$existing" == "$version_num" && "$force_install" != "true" ]]; then
        print_msg info "dotstate $existing is already installed (use --force to reinstall)"
        return 0
    fi

    if [[ -n "$existing" ]]; then
        if [[ "$existing" == "$version_num" ]]; then
            print_msg info "Reinstalling dotstate $version_num"
        else
            print_msg info "Upgrading from $existing to $version_num"
        fi
    fi

    # Construct download URL
    local download_url
    if [[ "$requested_version" == "latest" ]]; then
        download_url=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | \
            grep -o "\"browser_download_url\": \"[^\"]*${asset_name}\"" | \
            grep -v "\.sha256" | \
            cut -d '"' -f 4 | head -1)
    else
        download_url="https://github.com/${REPO}/releases/download/${requested_version}/${asset_name}"
    fi

    if [[ -z "$download_url" ]]; then
        print_msg error "Could not find download URL for $asset_name"
        return 1
    fi

    print_msg info "Downloading dotstate $version_num for $OS/$ARCH"

    mkdir -p "$INSTALL_DIR"

    local temp_dir=$(mktemp -d)
    local temp_file="$temp_dir/$asset_name"

    # Download
    if download_with_progress "$download_url" "$temp_file"; then
        cd "$temp_dir"
        tar xzf "$temp_file"

        local ext=""
        [[ "$OS" == "windows" ]] && ext=".exe"

        if [[ -f "${BINARY_NAME}${ext}" ]]; then
            mv "${BINARY_NAME}${ext}" "$INSTALL_DIR/${BINARY_NAME}${ext}"
            chmod +x "$INSTALL_DIR/${BINARY_NAME}${ext}"
        else
            print_msg error "Binary not found in archive"
            rm -rf "$temp_dir"
            return 1
        fi

        rm -rf "$temp_dir"
        return 0
    else
        print_msg error "Download failed"
        rm -rf "$temp_dir"
        return 1
    fi
}

print_banner() {
    echo ""
    echo -e "${CYAN}"
    echo -e "    ╺┳┓┏━┓╺┳╸┏━┓╺┳╸┏━┓╺┳╸┏━╸"
    echo -e "     ┃┃┃ ┃ ┃ ┗━┓ ┃ ┣━┫ ┃ ┣╸ "
    echo -e "    ╺┻┛┗━┛ ╹ ┗━┛ ╹ ╹ ╹ ╹ ┗━╸"
    echo ""
}

main() {
    print_banner

    if download_binary; then
        configure_path
        echo ""
        print_msg success "✓ Installation complete!"
        echo ""
        echo -e "  ${DIM}Run${NC} dotstate ${DIM}to get started${NC}"
        echo -e "  ${DIM}Docs:${NC} https://dotstate.serkan.dev"
        echo ""
        exit 0
    fi

    # Fallback to Cargo
    print_msg warn "Binary download failed, trying Cargo..."

    if install_via_cargo; then
        echo ""
        print_msg success "✓ Installed via Cargo!"
        echo ""
        exit 0
    fi

    echo ""
    print_msg error "Installation failed"
    print_msg info "Check: https://github.com/${REPO}/releases"
    print_msg info "Or install Rust: https://rustup.rs"
    exit 1
}

main
