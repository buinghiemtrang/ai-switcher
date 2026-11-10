# AI switcher changes

## 0.12.2 Linux preview

- Recognize the existing agy CLI login via Linux SecretService and read live model quotas from Google.
- Match tray tools to Claude, Codex and Antigravity; remove Cursor/opencode and show accounts even when a CLI binary is missing.
- Open the All quotas floating window directly from All or tray; default to all supported accounts.
- Keep agy account switching explicit: current CLI login is read-only; additional CLI profile switching is not supported.

## 0.12.1 Linux preview

- Added the All tab for Claude Code, Codex and Antigravity account quotas.
- Hidden Cursor CLI and opencode from the interface.
- Keep inactive Antigravity quotas cached and discard live results if the IDE account changes during refresh.
- Added a GitHub-backed npx installer with checksum verification.
- Simplified README into npx and Debian installer options; moved developer instructions to DEVELOPMENT.md.

## 0.12.0 Linux adaptation (preview)

- Independent app identity and local storage.
- Ubuntu package workflow and Linux CLI login terminals.
- Linux Claude credential-file detection.
- Removed upstream promotional link and legacy daemon cleanup.
- Experimental Antigravity Linux detection, capture and switching with private SQLite backups and atomic identity updates.
- Require the Linux IDE to be closed before identity changes; preserve account data on transaction failure.
