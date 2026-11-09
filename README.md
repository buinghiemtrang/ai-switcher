# AI switcher

Independent internal desktop build of an AI coding account manager, built with Tauri 2, Rust and React. This workspace targets Ubuntu 24.04 x86_64. Linux support is being adapted and must pass the checks below before distributing it to colleagues.

## Develop on Ubuntu

Install Node.js 20+, a current stable Rust toolchain, and the native build dependencies:

```bash
sudo apt update
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev patchelf curl sqlite3
npm ci
npm run tauri dev
```

A desktop session and a terminal emulator are required for interactive CLI sign-in. Install the AI CLIs you intend to manage separately.

```bash
npm run build
cd src-tauri
cargo test
cd ..
npm run tauri build -- --bundles deb
```

The Debian package is written beneath `src-tauri/target/release/bundle/deb/`. Build and smoke-test the package on the same Ubuntu version used by colleagues before sharing it. `npm run dev` alone is a browser preview with mock accounts; it does not verify native account switching.

## Capabilities and local data

- Manage isolated profiles for AI coding CLI accounts and select the active profile.
- Read provider quota, display usage/cost estimates and offer a local API gateway.
- Account credentials remain on the user's machine; requests to providers use those credentials.
- Selecting accounts can install shell hooks in `.bashrc`/`.zshrc` and per-account launchers in `~/.local/bin`.
- Default CLI profiles and project histories can be shared with managed profiles. Distribute the application package or clean source only, never your app-data folder, account profiles, API keys or CLI credentials.
- Antigravity account switching is currently unavailable on Linux. CLI login, quota and account switching still require native verification before rollout.

## Network behavior

The reviewed application source calls the configured AI providers (`api.anthropic.com`, `chatgpt.com`, `cursor.com`, `opencode.ai`), a local Antigravity service, and user-configured API/proxy endpoints. Cost estimation downloads the public LiteLLM pricing JSON from `raw.githubusercontent.com/BerriAI/litellm`; that download does not include account credentials. AI CLI login commands may open provider authentication pages independently.

The source review found no telemetry SDK, automatic application updater, or runtime upload endpoint belonging to the original author. This is a source review, not a guarantee about dependencies or future modifications. The former promotional blog button has been removed.

The optional gateway defaults to `127.0.0.1:8783` and uses gateway API keys. Binding it to `0.0.0.0` exposes an HTTP service to the network; use company-managed network protection if enabling that mode. Each employee can run their own local installation without sharing credentials.

## Internal distribution

The included GitHub workflow runs manually, builds an Ubuntu Debian package and stores it as a workflow artifact. It does not publish a release automatically. Review and test each artifact before distributing it through your company's chosen channel.

Before a rollout, verify startup, CLI detection, login, quota refresh, account switching, launchers, gateway authorization, and restart persistence using dedicated test accounts. Real provider authentication cannot be verified without those accounts.

## License

This fork retains the original MIT license and copyright notice in [LICENSE](LICENSE). Keep that file with source distributions and substantial copies of the software.
