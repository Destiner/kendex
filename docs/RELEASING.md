# Releasing

One workflow, tag-driven:

```
git tag v0.1.0 && git push origin v0.1.0
```

`.github/workflows/release.yml` builds on a native runner per target
(linux x86_64, macOS aarch64, windows x86_64) and publishes a **draft**
GitHub Release with:

- `kendex-<target>[.exe]` — the CLI binary, one per target. These are what
  `kendex update` downloads.
- The desktop app bundles Tauri produces per platform (deb/rpm/AppImage,
  dmg, NSIS installer).
- `feed.json` — the update feed `kendex update` reads from
  `releases/latest/download/feed.json`. Publishing the draft is what makes
  a version "latest"; until then existing installs see nothing.

Review the draft, then publish it. That is the release.

## User-supplied gates

- **Updater signing** (`TAURI_SIGNING_PRIVATE_KEY`,
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repo secrets): without them the
  desktop bundles build unsigned and no Tauri updater artifacts are
  produced. CLI self-update is unaffected.
- **macOS notarization / Windows code signing**: not configured; add
  certificates before distributing outside GitHub Releases.

## Local packaging

`cd crates/app && ../../ui/node_modules/.bin/tauri build` produces
deb/rpm anywhere; the AppImage step needs FUSE2 for linuxdeploy and may
fail on non-Debian hosts — the release runner covers it.

## Version bumps

The workspace version in `Cargo.toml` and `crates/app/tauri.conf.json`
must match the tag (minus the `v`) — `kendex update` compares its build
version against the feed, so a mismatched tag ships a feed that either
no-ops or loops.
