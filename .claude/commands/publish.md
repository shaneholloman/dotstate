---
description: Publish a new DotState release (bump version, validate, update changelog, commit, tag)
argument-hint: [patch|minor|major]
allowed-tools: Read, Edit, Write, Bash, Grep, Glob
---

# DotState Release Workflow

Perform a release for DotState with the following steps:

## Step 1: Determine Version Bump

Current version: !`grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/'`

Parse the bump type from argument: **$1** (default: patch)

- `patch`: 0.2.10 → 0.2.11
- `minor`: 0.2.10 → 0.3.0
- `major`: 0.2.10 → 1.0.0

Calculate and display the new version number.

## Step 2: Check for Uncommitted Code Changes

Run `git status` to check for uncommitted changes.

If there are uncommitted code changes (not just Cargo.toml/CHANGELOG.md):

1. **Commit code changes first** with a descriptive commit message that reflects what the changes do (e.g., "fix: add symlink validation to prevent crashes", "feat: add dark mode support")
2. Do NOT include these in the release commit - they should be separate commits

This ensures the git history clearly shows what code changes were made vs. what was just a version bump.

## Step 3: Run Sanity Checks

Execute all validation checks. If ANY fail, stop and report the issues:

```bash
cargo fmt --check    # Check formatting
cargo clippy         # Check lints (must have zero warnings)
cargo test           # Run all tests
cargo build          # Ensure it builds
```

Report results for each check. If all pass, continue. If any fail, stop and provide guidance on fixing.

## Step 4: Check Documentation and Website

Before releasing, check if documentation or website needs updates:

1. **README.md**: Does it reflect new features or changed behavior?

2. **Website** (`website/src/pages/index.astro`): The website is an Astro site with a TUI aesthetic. Check:
   - **CLI Commands section**: Are all commands documented with correct flags?
     - Each command uses `<CommandCard command="..." description="..." options="..." />`
   - **Features section**: Are new features listed?
     - Uses `<FeatureCard icon="[x]" title="...">` components
   - **Installation section**: Are install methods current?
   - **Version in header**: Update version in `global.css` header chrome if needed

   Key website files:
   - `website/src/pages/index.astro` - Main content (all sections)
   - `website/src/components/` - Reusable components (CommandCard, CodeBlock, etc.)
   - `website/src/styles/global.css` - Styling and CSS variables

   To verify website changes: `cd website && npm run build && npm run preview`

3. **CLAUDE.md**: If architecture changed, does the dev guide need updates?

If documentation updates are needed, ask the user if they want to:

- Update docs now (before release)
- Skip and release anyway
- Cancel release to update docs separately

## Step 5: Bump Version in Cargo.toml

Update the version field in Cargo.toml to the new version.

## Step 6: Process Changelog

Read CHANGELOG.md and process the `[Unreleased]` section:

1. **Analyze entries**: Look at each entry under Added, Changed, Fixed, etc.
2. **Consolidate related entries**: If multiple entries describe related changes, merge them into a single concise entry. For example:
   - Multiple sync-related fixes → "Fixed sync reliability issues with git rebase workflow"
   - Multiple package manager fixes → "Fixed package manager installation and UI issues"
3. **Keep it concise**: Each category should have 2-5 bullet points max. Prefer overview descriptions over granular details.
4. **Preserve important details**: Don't lose information about new features or breaking changes.

Then update the changelog:

- Change `## [Unreleased]` entries to `## [X.Y.Z] - YYYY-MM-DD` (use today's date)
- Add a fresh empty `## [Unreleased]` section at the top

Example structure after update:

```markdown
## [Unreleased]

---

## [0.2.11] - 2025-01-20

### Added

- ...

### Changed

- ...
```

## Step 7: Commit Version Bump

Stage and commit ONLY the version bump files, don't add Co-Author attribution:

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to v{VERSION}"
```

This commit should ONLY contain version-related changes, not code changes.

## Step 8: Create Git Tag

Create an annotated tag:

```bash
git tag -a v{VERSION} -m "Release v{VERSION}"
```

## Step 9: Summary

Display a summary:

- Previous version → New version
- Commits created (list each commit made during this release)
- Changes included (brief)
- Git tag created
- Remind: `git push && git push --tags` to publish

Do NOT push automatically - let the user review and push manually.
