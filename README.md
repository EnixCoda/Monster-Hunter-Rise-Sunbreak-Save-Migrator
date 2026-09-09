# Monster Hunter Rise Sunbreak Save Migrator

Migrate your **Monster Hunter Rise: Sunbreak** save from **Nintendo Switch → Steam PC** — characters, HR/MR, gear, talismans, decorations, quest progress and monster-size records. Everything runs 100% in your browser (WebAssembly); your save files never leave your machine. MIT-licensed fan tool.

## 🖥 Web app (recommended)
No install — the whole tool runs in the browser.

```bash
cd web
npm install
npm run dev        # localhost dev server
npm run build      # static site → web/dist
```

- Use **Chrome/Edge** (File System Access API) over **localhost/HTTPS**.
- The whole Steam save is **backed up automatically** before every migration; existing backups can be **restored** (undoable) or **deleted**.
- Final 3 slots are **reorderable** (↑/↓). By default your existing Steam characters stay in place and Switch characters are appended after.

## 💻 CLI (macOS/Linux)
Same migration from the terminal via `migrate_switch_to_steam.sh` (usage in its header). Uses the prebuilt `save_copy` — rebuild from `src/save_copy.rs` if needed.

## 🔁 Steam → Steam (account transfer)
Move a character from one Steam account to another with `tools/steam2steam.js` — same class-copy approach as Switch → Steam, plus the account-identity fixes the game validates (SteamID64, character GUID `e40fc0cd`, sys hunter-entry display values and the `eec7904b` appearance class). Usage and mechanism: [`tools/steam2steam.md`](./tools/steam2steam.md).

## Tests
```bash
cd tests && ./run_tests.sh           # CLI self-checks
cd web && npx playwright test e2e/   # web e2e (16 specs)
```

## License
[MIT](./LICENSE). Derived from [kvasszn/ree-save-editor](https://github.com/kvasszn/ree-save-editor) (upstream has no explicit license). Unofficial fan tool, not affiliated with or endorsed by CAPCOM.
