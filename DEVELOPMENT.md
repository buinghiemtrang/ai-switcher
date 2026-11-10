# Phát triển AI switcher

[← Cài đặt và sử dụng](README.md)

## Develop on Ubuntu

Clone your repository first (private access requires GitHub authentication):

```bash
git clone https://github.com/buinghiemtrang/ai-switcher.git
cd ai-switcher
```


Install Node.js 20+, a current stable Rust toolchain, and the native build dependencies:

```bash
sudo apt update
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libdbus-1-dev patchelf curl sqlite3 lsof zsh
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
- Antigravity Linux capture/switching is experimental and requires `sqlite3`. Close all IDE windows before switching or starting a new login; the app backs up the database before changing the token and avatar together. Actual provider login and switching require verification with your installed IDE version before rollout.

## Network behavior

The reviewed application source calls the configured AI providers (`api.anthropic.com`, `chatgpt.com`, `cursor.com`, `opencode.ai`), a local Antigravity IDE service, Google Antigravity CLI quota APIs (`cloudcode-pa.googleapis.com`), and user-configured API/proxy endpoints. Cost estimation downloads the public LiteLLM pricing JSON from `raw.githubusercontent.com/BerriAI/litellm`; that download does not include account credentials. AI CLI login commands may open provider authentication pages independently.

The source review found no telemetry SDK, automatic application updater, or runtime upload endpoint belonging to the original author. This is a source review, not a guarantee about dependencies or future modifications. The former promotional blog button has been removed.

The optional gateway defaults to `127.0.0.1:8783` and uses gateway API keys. Binding it to `0.0.0.0` exposes an HTTP service to the network; use company-managed network protection if enabling that mode. Each employee can run their own local installation without sharing credentials.

## Internal distribution

The GitHub workflow builds and tests on pushes and pull requests, then stores the Debian package as an artifact. To publish a downloadable preview installer, run the workflow manually with a new `release_tag` (for example `v0.12.2-linux.1`). The release job runs only after build and tests succeed. A blank tag only builds an artifact. Review and test installers before company rollout.

Before a rollout, verify startup, CLI detection, login, quota refresh, account switching, launchers, gateway authorization, and restart persistence using dedicated test accounts. Real provider authentication cannot be verified without those accounts.

## Verification of Linux preview

Commit `39eed0b` passed the frontend build, all 112 Rust tests, and Ubuntu Debian packaging in GitHub Actions run `42605079746`. The native application was started on the development desktop with no startup error in the captured log. Actual UI interactions and real provider authentication/account switching still require testing, especially Antigravity with the installed IDE version. The installer includes the MIT notice at `/usr/share/doc/ai-switcher/copyright`.

## Phát hành và cập nhật launcher npx

1. Cập nhật version trong `package.json`/`package-lock.json`, `src-tauri/tauri.conf.json` và `src-tauri/Cargo.toml`/`Cargo.lock` bằng công cụ quản lý tương ứng.
2. Đổi hằng `RELEASE` trong `bin/ai-switcher.cjs` sang tag mới và cập nhật link/tên file `.deb` trong README.
3. Commit và push source, rồi chạy workflow **Build Ubuntu package** với `release_tag` trùng hằng trên.
4. Workflow chạy test, build `.deb`, chuẩn hóa tên file và tạo `SHA256SUMS`, sau đó xuất bản GitHub prerelease.
5. Kiểm tra cài thật bằng `npx --yes github:buinghiemtrang/ai-switcher --install-only` và mở app trước khi chia sẻ.

Launcher dùng trực tiếp source GitHub, không cần publish package lên npm registry. Khi chỉ muốn cài trước mà chưa mở app, dùng `--install-only`; dùng `--update` để tải lại release được ghim trong launcher.

## Quota trong tab All

All chỉ hiển thị Claude Code, Codex và Antigravity. Cursor/opencode vẫn được giữ trong dữ liệu/backend, nhưng không hiển thị trong các tab chính.

Quota Antigravity lấy từ phiên `agy` CLI hiện tại trên Linux hoặc IDE đang chạy. Profile IDE chưa active giữ quota đã lưu; backend kiểm tra danh tính trước khi đọc và trước khi ghi kết quả để tránh gán quota sang tài khoản khác.

## Kiểm chứng 0.12.1

Source native `984ff76` đã qua 113 test Rust, 5 test installer Node và build gói Debian trong [Actions run 42612378745](https://github.com/buinghiemtrang/ai-switcher/actions/runs/42612378745).

Đã thử cài thật bằng `npx --yes github:buinghiemtrang/ai-switcher --install-only`, mở bằng đúng lệnh npx trong README, xác nhận All là màn hình mặc định, Refresh all hoàn tất và sidebar không có Cursor/opencode. Đăng nhập và chuyển tài khoản Antigravity thật vẫn chưa được kiểm chứng trên máy này.

## Antigravity CLI và quota nổi (0.12.2)

- Nhận diện binary `agy`, đọc phiên hiện tại từ SecretService bằng attributes service=gemini, username=antigravity. Không đọc nhầm Gemini OAuth file, không ghi/refresh credential trong app.
- Production `loadCodeAssist` trả project, `fetchAvailableModels` nhận project đó và trả quota theo model. Plan ưu tiên paidTier. API dùng access token hiện có, hết hạn thì mở agy để gia hạn.
- Machine default CLI là read-only. Tài khoản IDE đã lưu giữ riêng; chưa hỗ trợ chuyển nhiều account CLI.
- Tray chỉ Claude/Codex/Antigravity, không phụ thuộc installed flag để hiện các account đã biết. Antigravity mở main app từ tray thay vì tự đóng IDE.
- All quotas window dùng overlay có sẵn: mặc định tất cả account chưa ẩn của ba tool, chọn cũ chỉ có Cursor/opencode tự về tất cả. Nút ở All mở cửa sổ với toàn bộ account.
