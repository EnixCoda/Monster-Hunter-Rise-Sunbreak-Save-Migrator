# Steam -> Steam character migration

`tools/steam2steam.js` moves one character from a source Steam account to a
target Steam account. It is the Steam -> Steam equivalent of the Switch ->
Steam flow (`migrate_switch_to_steam.sh` / web tool), plus the identity fixes
the game validates when the title screen loads the character.

## Why the extra steps are needed

The Switch -> Steam flow works because a Switch save simply does not contain
the PC account/identity fields, so the game has nothing to validate. A PC ->
PC copy does carry them, and the game rejects the save when they do not match
the target account:

| field | where | why it must be fixed |
|---|---|---|
| SteamID64 | slot, 2 occurrences | rewritten to the target account id |
| `e40fc0cd` (16-byte char GUID) | slot + sys hunter entry | must be equal; set the slot's to the target sys entry's |
| hunter entry display values | sys | name / HR / MR / money / save-time / progress array |
| appearance class `eec7904b` | sys hunter entry | gender / face / hair / voice / colours |
| link-derived enums | sys hunter entry | must stay the **target** sys's value (`link << 8`) — never copy from source |

The sys patches are **value-only, in-place** (same sizes): no byte shifts, no
re-serialisation, so the sys structure stays exactly as the game wrote it.

## Pipeline

```
migrate_x(srcSlot -> target template)      # copy all 34 classes, re-key to target
  -> patch SteamID64 occurrences           # source id -> target id
  -> patch slot char GUID                  # target sys hunter entry GUID
  -> patch target sys hunter entry         # display values + appearance class
  -> verify (rank, GUID match, no source id, names)
```

Inputs:

- `srcSys` / `srcSlot` / `srcId` / `srcCurve` — source account files and SteamID64
- `tgtSys` / `tgtSlot` / `tgtId` / `tgtCurve` — target account files and SteamID64
  (`tgtSlot` is the slot being replaced; it only supplies the account-level classes)
- `srcCharIdx` / `tgtCharIdx` — hunter entry indices (0-based)

Outputs: new `data001Slot.bin` and `data00-1.bin`.

## CLI

```sh
cd mhrise-save-migrate
node tools/steam2steam.js \
  --src-sys  <src>/data00-1.bin    --src-slot <src>/data002Slot.bin \
  --src-id   <source-steamid64>    --src-curve 107 --src-char 1 \
  --tgt-sys  <tgt>/data00-1.bin    --tgt-slot <tgt>/data001Slot.bin \
  --tgt-id   <target-steamid64>    --tgt-curve 77  --tgt-char 0 \
  --out-dir  out/
```

The command prints a report (patched fields, rank, names) and the md5 of both
outputs. It throws if verification fails.

## Deploy (example: Steam Cloud remote folder on a LAN PC)

```sh
HOST=<user>@<host>
DIR='C:/Program Files (x86)/Steam/userdata/<target-account-id>/1446780/remote/win64_save'
STAMP=$(date +%Y%m%d_%H%M%S)

ssh $HOST "mkdir \"$DIR/../_mhr_backups/pre_s2s_$STAMP\" && \
  copy /Y \"$DIR\\data00-1.bin\" \"$DIR\\..\\_mhr_backups\\pre_s2s_$STAMP\\data00-1.bin\" && \
  copy /Y \"$DIR\\data001Slot.bin\" \"$DIR\\..\\_mhr_backups\\pre_s2s_$STAMP\\data001Slot.bin\""

scp out/data001Slot.bin $HOST:s2s_slot.bin
scp out/data00-1.bin    $HOST:s2s_sys.bin
ssh $HOST "copy /Y \"%USERPROFILE%\\s2s_slot.bin\" \"$DIR\\data001Slot.bin\" && \
           copy /Y \"%USERPROFILE%\\s2s_sys.bin\"  \"$DIR\\data00-1.bin\" && \
           del \"%USERPROFILE%\\s2s_slot.bin\" \"%USERPROFILE%\\s2s_sys.bin\""
```

Always start the game once with the character list visible and confirm the
list values before entering, then load the character.

## Notes

- The game re-saves with its own writer after you play: file bytes, the link
  (`0x0C`) and the sys hunter GUID layout can change. That is normal — the
  content stays correct. Re-running the pipeline uses the current target files.
- Name changes only patch in place when the encoded length is identical.
  A different-length name needs a length-neutral edit (renaming in game is
  the safe option).
- The source account's HunterUniqueID (`0a960102`) and NsaID (`cce8505f`) are
  intentionally left as-is; they are not cross-checked by the game.
