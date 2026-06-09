cask "agent-hub" do
  # ---------------------------------------------------------------------------
  # ASSUMPTIONS / NOTES
  #
  # 1. Asset naming follows tauri-action's macOS DMG output. Because the Tauri
  #    `productName` is "Agent Hub" (with a space), tauri-action substitutes the
  #    space with a "." in the DMG filename, e.g.:
  #        Agent.Hub_0.1.0_aarch64.dmg   (Apple Silicon)
  #        Agent.Hub_0.1.0_x64.dmg       (Intel)
  #    Confirm the exact names on a real GitHub Release and adjust if needed.
  #    tauri-action uses `aarch64` for Apple Silicon and `x64` for Intel DMGs.
  #
  # 2. The release tag is "v#{version}" (e.g. v0.1.0); the asset filenames do
  #    NOT carry the leading "v".
  #
  # 3. The GUI bundles the `agent-hub` CLI inside the .app at
  #    Contents/MacOS/agent-hub. The `binary` stanza symlinks it into the Cask
  #    bin dir so the CLI/TUI work from any terminal after install. If the CLI
  #    is shipped as a separate helper binary, change the source path below.
  #
  # 4. App is likely NOT notarized/signed. See the `caveats` block for the
  #    Gatekeeper quarantine workaround users may need.
  # ---------------------------------------------------------------------------

  version "0.1.0"

  on_arm do
    sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    url "https://github.com/xiaoleiy/agent-hub/releases/download/v#{version}/Agent.Hub_#{version}_aarch64.dmg"
  end

  on_intel do
    sha256 "1111111111111111111111111111111111111111111111111111111111111111"
    url "https://github.com/xiaoleiy/agent-hub/releases/download/v#{version}/Agent.Hub_#{version}_x64.dmg"
  end

  name "Agent Hub"
  desc "Menu-bar, CLI, and TUI tool to monitor and manage AI coding agents"
  homepage "https://github.com/xiaoleiy/agent-hub"

  livecheck do
    url :url
    strategy :github_latest
  end

  auto_updates false
  depends_on macos: :catalina

  app "Agent Hub.app"

  # Expose the bundled CLI/TUI binary as `agent-hub` on the user's PATH.
  binary "#{appdir}/Agent Hub.app/Contents/MacOS/agent-hub"

  zap trash: [
    "~/.agent-hub",
    "~/Library/Application Support/com.xiaoleiy.agent-hub",
    "~/Library/Caches/com.xiaoleiy.agent-hub",
    "~/Library/Preferences/com.xiaoleiy.agent-hub.plist",
    "~/Library/Saved Application State/com.xiaoleiy.agent-hub.savedState",
  ]

  caveats <<~EOS
    Agent Hub is currently distributed unsigned / not notarized. If macOS
    Gatekeeper blocks the app on first launch, remove the quarantine flag:

      xattr -dr com.apple.quarantine "/Applications/Agent Hub.app"

    After install, the `agent-hub` CLI/TUI is available in your terminal:

      agent-hub status        # System + agent status
      agent-hub tui           # Interactive dashboard
      agent-hub --help        # All subcommands
  EOS
end
