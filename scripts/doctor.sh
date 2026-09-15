#!/usr/bin/env bash
# Checks the developer's MacBook has the tools needed to build Vidya.
# Usage: ./scripts/doctor.sh
set -u
ok=0; bad=0
check() { # name, command, hint
  if eval "$2" >/dev/null 2>&1; then
    printf "  ✓ %-28s %s\n" "$1" "$(eval "$2" 2>/dev/null | head -n1)"; ok=$((ok+1))
  else
    printf "  ✗ %-28s %s\n" "$1" "$3"; bad=$((bad+1))
  fi
}
echo "Vidya doctor — $(uname -s) $(uname -m)"
echo
echo "Core tools"
check "Xcode command line tools" "xcode-select -p" "Run: xcode-select --install"
check "Homebrew"                 "brew --version" "Install from https://brew.sh"
check "Git"                      "git --version" "Run: brew install git"
check "Node.js"                  "node --version" "Run: brew install node (LTS)"
check "npm"                      "npm --version" "Comes with Node.js"
check "Rust (rustc)"             "rustc --version" "Install from https://rustup.rs"
check "Cargo"                    "cargo --version" "Comes with Rust"
check "Rust target Apple Silicon" "rustup target list --installed | grep -q aarch64-apple-darwin && echo installed" "Run: rustup target add aarch64-apple-darwin"
check "Rust target Intel Mac"    "rustup target list --installed | grep -q x86_64-apple-darwin && echo installed" "Run: rustup target add x86_64-apple-darwin"
echo
echo "Android"
check "ANDROID_HOME set"         "test -n \"\${ANDROID_HOME:-}\" && echo \$ANDROID_HOME" "Install Android Studio, then add ANDROID_HOME to ~/.zshrc (P8.1 explains)"
check "NDK installed"            "test -n \"\${NDK_HOME:-}\" && test -d \"\$NDK_HOME\" && echo \$NDK_HOME" "Install NDK in Android Studio SDK Manager, set NDK_HOME (P8.1)"
check "Java"                     "java -version 2>&1 | head -n1" "Use the JDK bundled with Android Studio (P8.1)"
check "adb"                      "adb version | head -n1" "Add \$ANDROID_HOME/platform-tools to PATH"
check "Rust target Android arm64" "rustup target list --installed | grep -q aarch64-linux-android && echo installed" "Added in P8.1"
echo
echo "Helpful tools (installed during the build steps)"
check "cargo-deny"               "cargo deny --version" "Run: cargo install cargo-deny --locked (P1.1)"
check "cargo-bloat"              "cargo bloat --version" "Run: cargo install cargo-bloat --locked (P10.1)"
check "GitHub CLI"               "gh --version" "Optional: brew install gh"
check "Claude Code"              "claude --version" "Install Claude Code (see Anthropic docs)"
echo
echo "Result: $ok found, $bad missing"
