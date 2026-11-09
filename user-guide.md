# AI switcher — Ubuntu preview

See [README](README.md) for setup and current verification status.

1. Install the Claude Code, Codex, Cursor CLI or opencode CLI you use.
2. Open AI switcher, inspect detected tool paths, then add a separate account profile.
3. Complete provider login in the terminal opened by the app.
4. Refresh quota and select the account. Open a new terminal or run `aisw` to synchronize existing shells.
5. Additional accounts receive dedicated launcher commands in `~/.local/bin`.

Credentials stay on each employee's computer. Share the application installer, not profile folders.
Antigravity Linux support is experimental: save the signed-in account, close all IDE windows before switching, then let AI switcher reopen it. A private SQLite backup is made before each identity change.
