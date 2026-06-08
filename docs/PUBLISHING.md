## Publishing Agent Hub via Homebrew Tap

### One-time setup (first release only)

1. **Create the tap repository.** Create a public GitHub repo named exactly
   `xiaoleiy/homebrew-tap` (the `homebrew-` prefix lets `brew tap xiaoleiy/tap`
   resolve to it). Add a `Casks/` directory and commit the `Casks/agent-hub.rb`
   cask file (placeholder version/sha256 are fine — CI overwrites them).

2. **Create a Personal Access Token (PAT).** In GitHub > Settings > Developer
   settings, create a fine-grained PAT with **Contents: Read and write** on the
   `xiaoleiy/homebrew-tap` repo (or a classic token with `repo` scope). The
   default `GITHUB_TOKEN` cannot push to a *different* repo, so the PAT is
   required for the cross-repo push.

3. **Add the secret to the app repo.** In `xiaoleiy/agent-hub` > Settings >
   Secrets and variables > Actions, add a secret named **`TAP_GITHUB_TOKEN`**
   with the PAT value.

4. **The `update-tap` job is already wired** into `.github/workflows/release.yml`
   (it runs on the `release: published` event), so no workflow edit is needed.
   It just needs the `TAP_GITHUB_TOKEN` secret (step 3) and the tap repo
   (step 1). Seed the tap's `Casks/agent-hub.rb` from
   `packaging/homebrew/Casks/agent-hub.rb` in this repo.

### Per-release checklist

1. **Bump the version** in `src-tauri/tauri.conf.json` (and `package.json` /
   `Cargo.toml` if you keep them in sync). Commit to `main`.

2. **Tag and push** a SemVer tag prefixed with `v`:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
   This triggers the `build` job (tauri-action builds aarch64 + x86_64 and
   creates a **draft** GitHub Release with the `.dmg` / `.app.tar.gz` artifacts).

3. **Publish the release.** Open the draft Release on GitHub, review it, and
   click **Publish release** (keep "Set as pre-release" UNCHECKED). Publishing
   the non-prerelease release is what fires the `update-tap` job, which pushes
   the updated cask to `xiaoleiy/homebrew-tap`.

4. **Verify the tap update.** Confirm a new commit `agent-hub <version>` landed
   in `xiaoleiy/homebrew-tap` and that `Casks/agent-hub.rb` has the new version
   and the two real `sha256` values.

5. **Test the install end-to-end:**
   ```bash
   brew untap xiaoleiy/tap 2>/dev/null || true
   brew tap xiaoleiy/tap
   brew install --cask xiaoleiy/tap/agent-hub
   agent-hub --version        # CLI symlink works
   open -a "Agent Hub"        # GUI launches
   ```
   For upgrades: `brew upgrade --cask agent-hub`.
   To audit the cask locally before tagging: `brew audit --cask --new agent-hub`
   and `brew style xiaoleiy/tap`.

### Caveats

- **Unsigned / not notarized:** Unless you set up an Apple Developer ID
  signing certificate + notarization in tauri-action, the `.app` is unsigned.
  Users will hit Gatekeeper ("can't be opened because Apple cannot check it").
  The cask `caveats` block documents the `xattr -dr com.apple.quarantine`
  workaround. Code signing + notarization is recommended but optional.
- **Asset naming:** The cask + tap job assume tauri-action's DMG names
  `Agent.Hub_<version>_aarch64.dmg` and `Agent.Hub_<version>_x64.dmg` (the
  space in "Agent Hub" becomes "."). Verify the actual filenames on the first
  real Release and adjust the cask URLs and the `gh release download` /
  `sha256sum` patterns in the tap job if they differ.
- **Draft vs published:** tauri-action sets `releaseDraft: true`, so releases
  are NOT auto-published. The tap update only runs after you manually publish,
  which is the intended gate. If you switch to auto-publish, the same `if`
  guard still skips prereleases.
- **CLI location:** The `binary` stanza assumes the CLI lives at
  `Contents/MacOS/agent-hub` inside the app bundle. If the bundled CLI is a
  separate helper or named differently, update the `binary` source path.
