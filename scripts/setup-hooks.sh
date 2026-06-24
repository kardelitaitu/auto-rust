#!/bin/sh
# setup-hooks.sh — Install git hooks from scripts/ into .git/hooks/
#
# Usage:
#   ./scripts/setup-hooks.sh          # Install all hooks (default)
#   ./scripts/setup-hooks.sh --list   # List available hooks
#   ./scripts/setup-hooks.sh pre-commit   # Install a specific hook only
#
# Hook source files live in scripts/ and are version-controlled.
# The installer copies (or symlinks, where supported) them to .git/hooks/.
# Edit scripts/<hook-name>, then re-run this script to apply changes.

set -e

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
HOOK_SOURCE_DIR="$REPO_ROOT/scripts"
HOOK_TARGET_DIR="$REPO_ROOT/.git/hooks"

# ---- Helpers -----------------------------------------------------------

green() { printf '\033[32m%s\033[0m\n' "$1"; }
red()   { printf '\033[31m%s\033[0m\n' "$1"; }
cyan()  { printf '\033[36m%s\033[0m\n' "$1"; }
bold()  { printf '\033[1m%s\033[0m\n' "$1"; }

usage() {
    bold "Usage:"
    echo "  $0                   Install all hooks"
    echo "  $0 --list            List available hooks"
    echo "  $0 <hook-name> ...   Install specific hooks only"
    echo ""
    bold "Available hooks:"
    for pattern in pre-commit* commit-msg* prepare-commit-msg* post-commit* pre-push*; do
        for hook in "$HOOK_SOURCE_DIR"/$pattern; do
            [ -f "$hook" ] && echo "  $(basename "$hook")"
        done
    done
    exit 0
}

# ---- Argument parsing --------------------------------------------------

if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    usage
fi

if [ "$1" = "--list" ]; then
    bold "Installed hooks in $HOOK_TARGET_DIR:"
    for pattern in pre-commit* commit-msg* prepare-commit-msg* post-commit* pre-push*; do
        for hook in "$HOOK_TARGET_DIR"/$pattern; do
            if [ -f "$hook" ]; then
                echo "  $(basename "$hook")  (installed)"
            fi
        done
    done
    echo ""
    bold "Available hooks in $HOOK_SOURCE_DIR:"
    for pattern in pre-commit* commit-msg* prepare-commit-msg* post-commit* pre-push*; do
        for hook in "$HOOK_SOURCE_DIR"/$pattern; do
            [ -f "$hook" ] && echo "  $(basename "$hook")"
        done
    done
    exit 0
fi

# Determine which hooks to install
if [ $# -eq 0 ]; then
    # Install all hooks found in scripts/
    HOOKS=""
    for pattern in pre-commit* commit-msg* prepare-commit-msg* post-commit* pre-push*; do
        for hook in "$HOOK_SOURCE_DIR"/$pattern; do
            [ -f "$hook" ] && HOOKS="$HOOKS $(basename "$hook")"
        done
    done
    set -- $HOOKS
fi

# ---- Validate git repo -------------------------------------------------

if [ ! -d "$HOOK_TARGET_DIR" ]; then
    red "Error: $HOOK_TARGET_DIR does not exist."
    red "Are you in the project root? Run from the repo root or use an absolute path."
    exit 1
fi

# ---- Install hooks -----------------------------------------------------

installed=0
skipped=0

for hook_name in "$@"; do
    source_path="$HOOK_SOURCE_DIR/$hook_name"
    target_path="$HOOK_TARGET_DIR/$hook_name"

    if [ ! -f "$source_path" ]; then
        red "  ✗  $hook_name — source not found at $source_path"
        continue
    fi

    # Detect whether we can symlink (Unix) or must copy (Windows / CI)
    use_symlink=true
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*)
            use_symlink=false
            ;;
    esac

    if $use_symlink; then
        ln -sf "$source_path" "$target_path" 2>/dev/null || use_symlink=false
    fi

    if ! $use_symlink; then
        cp "$source_path" "$target_path"
        chmod +x "$target_path"
    fi

    if [ -x "$target_path" ]; then
        if $use_symlink; then
            green "  ✓  $hook_name → symlinked -> .git/hooks/$hook_name"
        else
            green "  ✓  $hook_name → copied -> .git/hooks/$hook_name"
        fi
        installed=$((installed + 1))
    else
        red "  ✗  $hook_name — install failed"
        skipped=$((skipped + 1))
    fi
done

# ---- Summary -----------------------------------------------------------

echo ""
if [ $skipped -eq 0 ]; then
    green "  ✓  $installed hook(s) installed successfully."
    echo "     They will run automatically on 'git commit'."
    echo "     Edit scripts/$hook_name and re-run setup-hooks.sh to update."
else
    red "  ⚠  $installed installed, $skipped failed."
    exit 1
fi
