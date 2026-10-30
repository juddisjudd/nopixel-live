# Development

NoPixel Live is a [Tauri 2](https://v2.tauri.app/) app. The window is Svelte 5 and TypeScript. The backend is Rust.

## Requirements

- Windows 10 or 11.
- Node.js LTS and pnpm 11.
- Rust stable, 1.90 or later.
- VLC or mpv, to test playback. The app can install Streamlink itself.

## Run it

```sh
pnpm install
pnpm app          # dev mode with hot reload
pnpm tauri build  # installer in src-tauri/target/release/bundle/nsis
```

## Checks

```sh
pnpm check  # Svelte and TypeScript
pnpm lint   # clippy, warnings fail
pnpm test   # Rust unit tests
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored   # calls Twitch and Kick, downloads Streamlink
```

## Automatic builds

`.github/workflows/build.yml` runs on every push to `main`, every pull request, and on demand from the Actions tab. It runs the checks and builds the installer. The installer is saved on the run's page under **Artifacts**.

To publish a release:

1. Pick the next version, for example `0.2.0`.
2. Tag the commit and push the tag:

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

3. Wait about 20 minutes. The workflow builds the installer as version `0.2.0` and attaches it to a GitHub release named "NoPixel Live 0.2.0".

The tag sets the installer version. The version in `package.json`, `Cargo.toml`, and `tauri.conf.json` is only used for local builds. Tags must look like `v1.2.3`, or the build stops.

The installer is not code-signed, so Windows SmartScreen warns on the first run.

### Updates

Installed apps update themselves from `latest.json` on the newest GitHub release. The workflow writes that file on every tag build.

- Tag builds sign the installer with the updater key. The private key is the `TAURI_SIGNING_PRIVATE_KEY` repository secret. It has no password.
- A copy of the key is in `~/.tauri/nopixel-live.key`. Back it up. If the key is lost, installed apps cannot update, and every user must install the next version by hand.
- The public key is `plugins.updater.pubkey` in `tauri.conf.json`. Change it only together with the secret.
- Builds without a tag do not sign anything, so they need no key.

## How discovery works

Both sources are the endpoints that twitch.tv and kick.com use in the browser. They are not official APIs, so they can change without notice. If one breaks, the app shows the error under the header and keeps showing the other site.

- **Twitch** allows 30 results per query and blocks paging without a browser security token. The app asks for each tag sorted three ways (most viewers, fewest viewers, newest) in one request. This covers up to 60 streams per tag, plus some extra from the newest sort.
- **Kick** returns the whole event list in one request. The event belongs to the GTA V category, so its streams pass the category filter without a per-stream lookup.
- **Cross-site lookup:** every Kick streamer is also looked up on Twitch by name. If their Twitch stream title mentions NoPixel, the two entries are merged.

## Project layout

| Path | Role |
| --- | --- |
| `src-tauri/src/twitch.rs` | Twitch GraphQL query and parsing |
| `src-tauri/src/kick.rs` | Kick event and channel parsing |
| `src-tauri/src/merge.rs` | Joins channels into one streamer per person |
| `src-tauri/src/poller.rs` | Poll loop, favorite notifications |
| `src-tauri/src/player.rs` | Finds Streamlink, VLC, and mpv, and launches Streamlink |
| `src-tauri/src/install.rs` | Downloads, checks, and unpacks the portable Streamlink build |
| `src-tauri/src/chat.rs` | Opens popout chat in a window or the browser |
| `src-tauri/src/settings.rs` | Settings file, defaults, and input cleanup |
| `src/App.svelte` | Main window |
| `src/lib/StreamerTile.svelte` | One streamer |
| `src/lib/SettingsPanel.svelte` | Settings dialog with tabs |
| `src/lib/SetupWizard.svelte` | First-run setup: Streamlink, player, chat |
| `src/lib/Segmented.svelte` | Segmented radio buttons used by both dialogs |
| `src/lib/updater.svelte.ts` | Update checks and install |
