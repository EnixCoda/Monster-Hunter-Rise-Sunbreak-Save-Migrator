# Monster Hunter Rise Sunbreak Save Migrator

**Author:** EnixCoda

Easily **migrate** your **Monster Hunter Rise Sunbreak** saves from **Nintendo Switch to Steam PC** — validate, transfer, back up and restore. Everything runs in the browser (WebAssembly), private and offline; your save files never leave your machine.

## 🖥 Web app (recommended) — 100% in-browser, no install
The core is compiled to **WebAssembly** (`web/` → `wasm/`), so the whole tool runs
in the browser: open the site, pick your Switch save, pick your Steam folder
(File System Access API), validate, migrate. Files never leave your machine.

**Build + run locally:**
```bash
cd web
npm install
npm run build        # outputs static site to web/dist (open or host it)
npm run dev          # dev server (localhost) — or: npm run build && npx vite preview
```
- Use **Chrome or Edge** (File System Access API) for folder picking.
- Need secure context: `localhost` or any HTTPS host works (you can also drop it on
  GitHub Pages for a shareable "open the site" URL).

**Safety first — backups & restore:**
- Before **every** migration the whole Steam save (`data00-1.bin` + all slot
  files) is snapshotted into `backup_<timestamp>/` inside
  `win64_save/` — nothing is written until the backup completes.
- After picking your Steam folder, existing backups are listed in the app with
  **Restore** (which takes its own pre-restore snapshot first, so restores are
  undoable) and **Delete**.
- The final 3 slots can be **reordered** (↑/↓) instead of always putting Switch
  saves first.
- Needs Chrome/Edge (File System Access API) over localhost/HTTPS; the app shows
  a banner if the browser is unsupported.

## 💻 CLI alternative (macOS/Linux)
`migrate_switch_to_steam.sh` (+ `save_copy`) — same migration for users who prefer
a command line (also works where the binary runs). See its header for usage and
the "Files" / "How it works" / "Notes" sections below (unchanged).

Migrate a **Nintendo Switch** Monster Hunter Rise: Sunbreak save (JKSV dump)
onto a **Steam PC** save, preserving items, gear, talismans, decorations,
buddies, quests/story progression, HR/MR, guild card, and more.

The PC game accepts the result: the tool re-encrypts the slot save (Citrus),
and — the key fix — keeps the save **link** (`0x0C` header value) identical
between `data00-1.bin` and the slot file, plus corrects the murmur3 tail
checksum. Without the link fix the game silently rejects external saves.

## Files
- `migrate_switch_to_steam.sh` — the migration script (usage in its header)
- `save_copy` — prebuilt CLI (macOS/Linux x64). Windows: use WSL, or rebuild
  from `src/save_copy.rs` against the upstream
  [ree-save-editor](https://github.com/kvasszn/ree-save-editor) repo
  (this tool is derived from it; upstream has no license file).
- `assets/mhrise/enumsmhrise.json` — required runtime data
- `src/save_copy.rs` — the custom CLI source (reference; needs ree-lib to build)
- `tests/` — self-check test suite

## How it works
1. Loads your Steam save (must be **your own account** — create a fresh
   character first).
2. Copies **all** save classes from the Switch dump into it (slice per class):
   Mission/progression, FlagData, NPC guide, HunterRecord, Progress & quest
   flags, event flags, GuildCard, telemetry, DataManager (items/equip/deco/
   buddies/loadouts/layered box), System, FacilityDataManager, EquipDataManager,
   wish list, trade centre, dojo, etc.
3. Intentionally **excludes** account-bound data: network saves, DLC ownership,
   and HunterRecord online IDs (HunterUniqueID/NetworkUniqueId/NsaID) — avoids
   account binding / ban risk. HR/MR and all progress still transfer.
4. Re-links `data00-1.bin` ↔ slot (`0x0C` match) + fixes the murmur3 tail.
5. Backs up your originals first, then replaces the files.

## Usage
```bash
# 1) Prepare: your Steam save dir must have your own char created:
#    Steam/userdata/<accountid>/1446780/remote/win64_save/
# 2) Get your Switch dump (JKSV) -> data001Slot.bin
./migrate_switch_to_steam.sh \
    --switch /path/to/switch_data001Slot.bin \
    --steamid64 76561198XXXXXXXX \
    --pc-dir "C:/Program Files (x86)/Steam/userdata/<accountid>/1446780/remote/win64_save" \
    [--slot data001Slot.bin] \
    [--out-dir /path/to/backup-staging]
```
Then **launch the game** and load your character. Everything matches your
Switch save and persists across load/save (verified: the game keeps quests
open, HR/MR, all gear/items/buddies).

## Notes / caveats
- **Citrus Curve Index** is fixed at **107** in the prebuilt binary (covers the
  standard case). If the game gives an encryption error on load, note the
  curve index from [ree-save-editor](https://github.com/kvasszn/ree-save-editor)
  and rebuild `save_copy` with it.
- The PC slot file must be one attached to your account. Slot 1 (`data001Slot.bin`)
  is used by default; pass `--slot` for slot 2/3.
- Always keep the auto-created `backup_<timestamp>/` folder.
- These are **save files** — use at your own risk; keep backups.

## Tests
```bash
cd tests && ./run_tests.sh                 # CLI self-checks
cd web && npx playwright test e2e/        # web e2e (mock File System Access API)
```
The CLI suite self-checks the binary (header/link/murmur round-trip, slice
behavior) and, if fixture saves are present, a full migration + verification.
The e2e suite covers the full browser flow plus: backups (list/auto-create/
restore/delete), slot reordering, the >3-slot guard, corrupt saves, picking
cancellation, and the missing-API fallback message.

## License
Released under the **MIT License** (see [`LICENSE`](./LICENSE)). Attribution and
third-party/data notices are in [`NOTICE`](./NOTICE). This is an unofficial fan
tool for Monster Hunter Rise: Sunbreak, not affiliated with or endorsed by CAPCOM.
