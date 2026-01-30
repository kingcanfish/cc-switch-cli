#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 5 ]]; then
  echo "Usage: $0 <tag> <sha_macos_arm64> <sha_macos_x64> <sha_linux_x64> <sha_linux_arm64>" >&2
  exit 1
fi

TAG="$1"
SHA_MACOS_ARM64="$2"
SHA_MACOS_X64="$3"
SHA_LINUX_X64="$4"
SHA_LINUX_ARM64="$5"
VERSION="${TAG#v}"
REPO="kingcanfish/cc-switch-cli"

cat << FORMULA
class CcSwitchCli < Formula
  desc "Command-Line Management Tool for Claude Code, Codex & Gemini CLI"
  homepage "https://github.com/${REPO}"
  version "${VERSION}"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/${REPO}/releases/download/${TAG}/cc-switch-cli-${TAG}-darwin-arm64.tar.gz"
      sha256 "${SHA_MACOS_ARM64}"
    end
    on_intel do
      url "https://github.com/${REPO}/releases/download/${TAG}/cc-switch-cli-${TAG}-darwin-x64.tar.gz"
      sha256 "${SHA_MACOS_X64}"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/${REPO}/releases/download/${TAG}/cc-switch-cli-${TAG}-linux-arm64-musl.tar.gz"
      sha256 "${SHA_LINUX_ARM64}"
    end
    on_intel do
      url "https://github.com/${REPO}/releases/download/${TAG}/cc-switch-cli-${TAG}-linux-x64-musl.tar.gz"
      sha256 "${SHA_LINUX_X64}"
    end
  end

  def install
    bin.install "cc-switch-cli"
  end

  test do
    system "#{bin}/cc-switch-cli", "--help"
  end
end
FORMULA
