# Install channels

The headline install is the curl script; the rest are package-manager
entries that point at the same GitHub release artifacts.

| Channel | Command | Recipe |
|---|---|---|
| curl | `curl -fsSL https://kendex.ai/install.sh \| sh` | [`/install.sh`](../install.sh) |
| Homebrew | `brew install vanillagreencom/kendex/kendex` | [`homebrew/kendex.rb`](homebrew/kendex.rb) |
| Arch (prebuilt) | `yay -S kendex` | [`arch/kendex/`](arch/kendex/) |
| Arch (latest commit) | `yay -S kendex-git` | [`arch/kendex-git/`](arch/kendex-git/) |
| App bundles | download from the release | built by `release.yml` (`.dmg`/`.msi`/`.AppImage`/`.deb`/`.rpm`) |

The desktop app ships as native bundles attached to every release; only
the CLI is installed by curl, Homebrew, and AUR.

## Per release

Each new `vX.Y.Z` changes the artifact checksums. Update, in this repo:

- `arch/kendex/PKGBUILD` and `.SRCINFO`: `pkgver` and the `sha256sums`
  line (the released `kendex-x86_64-unknown-linux-gnu`).
- `homebrew/kendex.rb`: `version` and both `sha256` lines.

Then push the recipes to their channels:

- **Homebrew**: copy `homebrew/kendex.rb` into the tap repo
  `vanillagreencom/homebrew-kendex` and commit.
- **Arch**: in each AUR package clone, copy the `PKGBUILD` + `.SRCINFO`
  and `git push` to `ssh://aur@aur.archlinux.org/kendex.git` (and
  `kendex-git.git`). Pushing needs the AUR account's SSH key.

`kendex-git` needs no checksum change; its `pkgver()` is computed at build
time from the cloned commit.
