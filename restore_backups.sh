#!/bin/bash

# Restore dotfiles from .bak backup files
# This script finds all .bak files and restores them to their original locations

# Don't use set -e, we'll handle errors manually

# Change to home directory to ensure we're in the right context
cd ~ || exit 1

# echo "🧹 Cleaning up orphaned symlinks first..."

# # Find and remove ONLY orphaned symlinks (symlinks that point to non-existent files)
# # This must be done first because orphaned symlinks will make [ -e ] return true
# # We use test -e on the symlink which checks if the TARGET exists (not the symlink itself)
# orphaned_count=0
# while IFS= read -r symlink; do
#     # Only process if it's actually a symlink
#     if [ -L "$symlink" ]; then
#         # Check if the symlink's TARGET exists
#         # Using test -e on a symlink checks the target, not the symlink itself
#         # So if test -e returns false, the target doesn't exist (orphaned)
#         if ! test -e "$symlink"; then
#             # Double-check: read the link and verify target doesn't exist
#             target=$(readlink "$symlink" 2>/dev/null)
#             if [ -n "$target" ]; then
#                 # Resolve the target path
#                 symlink_dir=$(dirname "$symlink")
#                 if [ "${target#/}" = "$target" ]; then
#                     # Relative path - resolve from symlink's directory
#                     resolved_target=$(cd "$symlink_dir" 2>/dev/null && realpath "$target" 2>/dev/null || echo "")
#                 else
#                     # Absolute path
#                     resolved_target=$(realpath "$target" 2>/dev/null || echo "")
#                 fi

#                 # Only remove if target definitely doesn't exist
#                 if [ -z "$resolved_target" ] || [ ! -e "$resolved_target" ]; then
#                     if rm "$symlink" 2>/dev/null; then
#                         echo "🗑️  Removed orphaned symlink: $symlink -> $target"
#                         orphaned_count=$((orphaned_count + 1))
#                     fi
#                 fi
#             fi
#         fi
#     fi
# done < <(find . -type l 2>/dev/null || true)

# if [ $orphaned_count -gt 0 ]; then
#     echo "   Removed $orphaned_count orphaned symlink(s)"
# else
#     echo "   No orphaned symlinks found"
# fi

echo ""
echo "🔍 Searching for .bak backup files in home directory..."

# Find all .bak files (searching from home directory)
# Skip unnecessary directories to avoid going unnecessarily deep
backup_files=$(find . \
    -name "*.bak" \
    -type f \
    -not -path "./Library/*" \
    -not -path "./.Trash/*" \
    -not -path "./.deno/*" \
    -not -path "./code/*" \
    2>/dev/null || true)

if [ -z "$backup_files" ]; then
    echo "❌ No .bak files found in home directory"
    exit 1
fi

# Count backups
backup_count=$(echo "$backup_files" | wc -l | tr -d ' ')
echo "📦 Found $backup_count backup file(s):"
echo ""
# List all backup files
while IFS= read -r backup_file; do
    echo "   • $backup_file"
done <<< "$backup_files"
echo ""

# Ask for confirmation
read -p "⚠️  This will restore all .bak files. Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Cancelled"
    exit 1
fi

# Restore each backup
restored=0
skipped=0
errors=0

while IFS= read -r backup_file; do
    # Get original filename (remove .bak extension)
    original_file="${backup_file%.bak}"

    # Check if original already exists (and is not a broken symlink)
    # -e returns true for broken symlinks, so we check -f or -d to ensure it's real
    if [ -f "$original_file" ] || [ -d "$original_file" ]; then
        echo "⏭️  Skipping $original_file (already exists)"
        skipped=$((skipped + 1))
    else
        # Restore the backup
        if mv "$backup_file" "$original_file" 2>/dev/null; then
            echo "✅ Restored: $original_file"
            restored=$((restored + 1))
        else
            echo "❌ Failed to restore: $original_file"
            errors=$((errors + 1))
        fi
    fi
done <<< "$backup_files"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Summary:"
echo "   ✅ Restored: $restored"
echo "   ⏭️  Skipped: $skipped"
echo "   ❌ Errors: $errors"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $errors -gt 0 ]; then
    exit 1
fi

