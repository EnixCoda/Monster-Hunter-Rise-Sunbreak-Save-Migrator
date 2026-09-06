# size-sync

Surgically copy **only the monster-size records** from one MH Rise save's
`HunterRecord` into another, leaving every other field (HR/MR, hunting counts,
playtime, quests, items, all progress) untouched. The slot is re-linked and its
Murmur3 tail-hash recomputed so the game accepts it.

This was written because migrating a character already carries the `HunterRecord`
class, but the PC save had kept playing and recorded its *own* sizes; the user
wanted to overlay the Switch size set onto the live PC save without losing any
other progress.

## What it patches
Only these two arrays inside `snow.HunterRecordManager.HunterReocrdSaveData`:

| Field | murmur3(name, 0xffffffff) |
|---|---|
| `HunterRecord` (top-level class) | `0x355c8c4f` |
| `EnemySizeMin` | `0xa23ee890` |
| `EnemySizeMax` | `0xfac89643` |

`EnemySizeMin/Max` are the 120-element F32 per-monster size-ratio records
(the in-game "Hunter's Notes → Monster → Size" data).

Everything else in the target save is preserved exactly.

## Build

Prereqs: a Rust toolchain. `ree-lib` (the MH Rise save library) must be at
`/tmp/reefresh` — the same external dependency the `wasm` crate already uses.

```
cd tools/size-sync
cargo build --release
```

**macOS note:** `ree-lib`'s `Cargo.toml` only declares `num-bigint` /
`num-traits` / `num-integer` for `linux`, `windows` and `wasm32`, so a macOS
*host* build fails. Add this block to `/tmp/reefresh/Cargo.toml` once:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
num-bigint = "0.4.6"
num-traits = "0.2"
num-integer = "0.1"
```

## Usage

### Sync sizes (primary command)
```
size-sync <pc_slot.bin> <switch_slot.bin> <out.bin> <steamid64> [pc_data00-1.bin]
```
Example:
```
./target/release/size-sync \
  live_data002Slot.bin \
  "/path/to/Switch/data001Slot.bin" \
  patched_data002Slot.bin \
  <STEAMID64> \
  live_data00-1.bin
```

- `steamid64` must be the SteamID64 of the *target* (PC) save: `0x0110000100000000 + accountId`.
- The optional `pc_data00-1.bin` supplies the system-file link word so the slot
  is re-linked to the same system save.
- On success it prints `patched EnemySizeMin/Max`, re-loads the output
  (`verify reload OK`), and prints the preserved `HuntingCount` plus the new
  size-array head — so you can confirm progress was kept.

### Diagnostics (read-only)
```
size-sync list <file> <steamid>                       # top-level classes
size-sync clsdump <file> <steamid> <top-hash> [depth] # dump a class tree
```

## Safety
1. Always back up the target slot first (e.g. copy `data002Slot.bin` →
   `data002Slot.bin.before_sizesync`).
2. Only run while the game is **closed**; writing a live in-use save will be
   overwritten or rejected.
3. **Disable Steam Cloud** for the game (or go offline) before launching,
   otherwise Steam may restore its server copy and wipe the change.
4. Verify after writing by reading the file back (`size-sync clsdump ...` or the
   tool's own reload check) before playing.

## Notes / hashes
Other useful field hashes (murmur3(name, 0xffffffff)) used while diagnosing:

| Field | Hash |
|---|---|
| `ProgressSaveData` (top level) | `0x164d2e71` |
| `_HunterRank` | `0x5653c091` |
| `_MasterRank` | `0x4e428685` |
| `HuntingCount` | `0x1c4d0471` |
| `EnemyFindFlag` (128-bit bitmask class) | `0xaa7f7687` |
| `EnemyHuntingCount` | `0xfa78cb3b` |
| `EnemyCaptureCount` | `0x0d7b4dbb` |

All `murmur3(name, 0xffffffff)`; the tool computes `HuntingCount` at runtime the
same way.
