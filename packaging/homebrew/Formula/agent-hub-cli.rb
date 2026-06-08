# CLI/TUI-only install of Agent Hub — no GUI app, no sudo, no Gatekeeper.
# Installs just the `agent-hub` binary (extracted from the release bundle) into
# the Homebrew prefix, which is user-writable on Apple Silicon. Ideal for
# terminal users and IT-managed Macs without admin rights.
#
# Usage:  brew install xiaoleiy/tap/agent-hub-cli
#
# version + sha256 below are rewritten automatically by the release workflow.
class AgentHubCli < Formula
  desc "CLI + TUI to monitor AI coding agents (Claude Code, Cursor, Codex)"
  homepage "https://github.com/xiaoleiy/agent-hub"
  version "0.1.0"
  license "MIT"

  # Prebuilt binary extracted from the macOS release bundle — the same binary
  # serves the GUI, CLI, and TUI (it routes on its arguments).
  if Hardware::CPU.arm?
    url "https://github.com/xiaoleiy/agent-hub/releases/download/v#{version}/Agent.Hub_aarch64.app.tar.gz"
    sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  else
    url "https://github.com/xiaoleiy/agent-hub/releases/download/v#{version}/Agent.Hub_x64.app.tar.gz"
    sha256 "1111111111111111111111111111111111111111111111111111111111111111"
  end

  def install
    bin.install "Agent Hub.app/Contents/MacOS/agent-hub" => "agent-hub"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/agent-hub --version")
  end
end
