#!/bin/sh

set -eu

BUILD_PROFILE="${BUILD_PROFILE:-release}"
CODEX_CODE_HOME="${CODEX_CODE_HOME:-$HOME/.local/share/codex-personal}"
CODEX_CODE_INSTALL_DIR="${CODEX_CODE_INSTALL_DIR:-}"
PATH_ACTION="already"
PATH_PROFILE=""

step() {
  printf '==> %s\n' "$1"
}

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

repo_root() {
  script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
  CDPATH= cd -- "$script_dir/../.." && pwd
}

add_to_path() {
  install_dir="$1"

  PATH_ACTION="already"
  PATH_PROFILE=""

  case ":$PATH:" in
    *":$install_dir:"*)
      return
      ;;
  esac

  profile="$HOME/.profile"
  case "${SHELL:-}" in
    */zsh)
      profile="$HOME/.zshrc"
      ;;
    */bash)
      profile="$HOME/.bashrc"
      ;;
  esac

  PATH_PROFILE="$profile"
  path_line="export PATH=\"$install_dir:\$PATH\""
  if [ -f "$profile" ] && grep -F "$path_line" "$profile" >/dev/null 2>&1; then
    PATH_ACTION="configured"
    return
  fi

  if [ -e "$profile" ]; then
    [ -w "$profile" ] || {
      PATH_ACTION="failed"
      return
    }
  else
    profile_dir="$(dirname "$profile")"
    [ -w "$profile_dir" ] || {
      PATH_ACTION="failed"
      return
    }
  fi

  if {
    printf '\n# Added by Codex local code installer\n'
    printf '%s\n' "$path_line"
  } >>"$profile"; then
    PATH_ACTION="added"
  else
    PATH_ACTION="failed"
  fi
}

assert_macos_apple_silicon() {
  [ "$(uname -s)" = "Darwin" ] || fail "this installer only supports macOS"

  arch="$(uname -m)"
  if [ "$arch" = "arm64" ]; then
    return
  fi

  if [ "$arch" = "x86_64" ] && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = "1" ]; then
    fail "this shell is running under Rosetta; rerun from a native Apple Silicon terminal"
  fi

  fail "this installer only supports Apple Silicon macOS; detected architecture: $arch"
}

choose_link_dir() {
  if [ -n "$CODEX_CODE_INSTALL_DIR" ]; then
    mkdir -p "$CODEX_CODE_INSTALL_DIR"
    [ -w "$CODEX_CODE_INSTALL_DIR" ] || fail "install dir is not writable: $CODEX_CODE_INSTALL_DIR"
    printf '%s\n' "$CODEX_CODE_INSTALL_DIR"
    return
  fi

  existing_code="$(command -v code 2>/dev/null || true)"
  if [ -n "$existing_code" ]; then
    existing_dir="$(dirname "$existing_code")"
    if [ -w "$existing_dir" ]; then
      printf '%s\n' "$existing_dir"
      return
    fi
  fi

  for dir in /opt/homebrew/bin /usr/local/bin "$HOME/.local/bin" "$HOME/bin"; do
    if mkdir -p "$dir" 2>/dev/null && [ -w "$dir" ]; then
      printf '%s\n' "$dir"
      return
    fi
  done

  fail "could not find a writable install directory for the code command"
}

install_link() {
  target="$1"
  link_path="$2"

  if [ -L "$link_path" ]; then
    existing_target="$(readlink "$link_path" 2>/dev/null || true)"
    if [ "$existing_target" = "$target" ]; then
      step "$link_path already points to $target"
      return
    fi
    rm -f "$link_path"
  elif [ -e "$link_path" ]; then
    backup_path="$link_path.pre-codex-backup.$(date +%Y%m%d%H%M%S)"
    mv "$link_path" "$backup_path"
    step "Backed up existing code command to $backup_path"
  fi

  ln -s "$target" "$link_path"
  step "Linked $link_path -> $target"
}

assert_macos_apple_silicon

ROOT_DIR="$(repo_root)"
CODEX_RS_DIR="$ROOT_DIR/codex-rs"
INSTALL_BIN_DIR="$CODEX_CODE_HOME/bin"
LINK_DIR="$(choose_link_dir)"

case "$BUILD_PROFILE" in
  release)
    BUILD_DIR="$CODEX_RS_DIR/target/release"
    ;;
  dev | debug)
    BUILD_DIR="$CODEX_RS_DIR/target/debug"
    ;;
  *)
    fail "unsupported BUILD_PROFILE=$BUILD_PROFILE (expected release, dev, or debug)"
    ;;
esac

BUILT_BINARY="$BUILD_DIR/codex"
INSTALLED_BINARY="$INSTALL_BIN_DIR/codex"
LINK_PATH="$LINK_DIR/code"

[ -d "$CODEX_RS_DIR" ] || fail "missing codex-rs directory at $CODEX_RS_DIR"

step "Building codex-cli ($BUILD_PROFILE)"
(
  cd "$CODEX_RS_DIR"
  case "$BUILD_PROFILE" in
    release)
      cargo build --release -p codex-cli
      ;;
    dev | debug)
      cargo build -p codex-cli
      ;;
  esac
)

[ -x "$BUILT_BINARY" ] || fail "build succeeded but binary was not found at $BUILT_BINARY"

step "Smoke testing built binary"
"$BUILT_BINARY" --version >/dev/null

step "Installing built binary to $INSTALLED_BINARY"
mkdir -p "$INSTALL_BIN_DIR"
cp "$BUILT_BINARY" "$INSTALLED_BINARY"
chmod 0755 "$INSTALLED_BINARY"

step "Linking code to workspace build artifact"
install_link "$BUILT_BINARY" "$LINK_PATH"
add_to_path "$LINK_DIR"

case "$PATH_ACTION" in
  added)
    step "PATH updated for future shells in $PATH_PROFILE"
    ;;
  configured)
    step "PATH is already configured for future shells in $PATH_PROFILE"
    ;;
  failed)
    step "Could not update PATH automatically; add $LINK_DIR to your shell PATH manually"
    ;;
  *)
    step "$LINK_DIR is already on PATH"
    ;;
esac

step "Verifying linked command"
"$LINK_PATH" --version

printf '\n'
printf 'Installed local Codex fork as `code`.\n'
printf 'Run now: %s --help\n' "$LINK_PATH"
printf 'Run now: %s\n' "$LINK_PATH"
