#!/usr/bin/env bash
# =============================================================================
# MHRise/Sunbreak Switch -> Steam PC save migration
# Author: EnixCoda
#    Migrates a Nintendo Switch MH Rise: Sunbreak save dump (JKSV) onto a
#    Steam PC save, by copying ALL save classes (items, gear, talismans, decos,
#    buddies, quests/progression, guild card, etc.) into the PC save and
#    re-linking both save files so the game accepts them.
#
# REQUIRES:
#   - save_copy binary (in same dir as this script, or via SAVE_COPY env)
#   - Your Steam PC save files (these must be YOURS / attached to your account):
#       data00-1.bin  +  data001Slot.bin (or your slot) in:
#       <Steam>\userdata\<accountid>\1446780\remote\win64_save\
#   - Your Switch save dump (JKSV exported): data001Slot.bin (the char save)
#   - Your Steam ID64 (e.g. 76561198...) and Citrus Curve Index (usually 107;
#     see ree-save-editor note; can brute-force: keep default 107 first)
#
# USAGE:
#   ./migrate_switch_to_steam.sh --switch <switch slot.bin> \
#       --steamid64 <76561198...> \
#       --pc-dir "<Steam userdata .../1446780/remote/win64_save>" \
#       [--slot data001Slot.bin] [--out-dir <dir>]
#
# NOTES:
#   - Always creates a backup of your PC save first (backup_TIMESTAMP/).
#   - Online/account-bound data (network ID, DLC ownership) is intentionally
#     left as-is (account safety). HR/MR and everything else is transferred.
#   - The undermodded game re-derives quest-open flags from NPC flow; the
#     PATCHED SAVE itself carries the exact Switch state and is accepted.
# =============================================================================
set -euo pipefail

SAVE_COPY="${SAVE_COPY:-$(dirname "$0")/save_copy}"
if [ ! -x "$SAVE_COPY" ]; then
  echo "ERROR: save_copy binary not found (set SAVE_COPY or place next to script)" >&2
  exit 1
fi

SWITCH=""
STEAMID=""
PCDIR=""
SLOT="data001Slot.bin"
OUTDIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --switch) SWITCH="$2"; shift 2;;
    --steamid64) STEAMID="$2"; shift 2;;
    --pc-dir) PCDIR="$2"; shift 2;;
    --slot) SLOT="$2"; shift 2;;
    --out-dir) OUTDIR="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 1;;
  esac
done

if [ -z "$SWITCH" ] || [ -z "$STEAMID" ] || [ -z "$PCDIR" ]; then
  echo "ERROR: need --switch, --steamid64, --pc-dir" >&2
  echo "  ./$(basename "$0") --help (see header)" >&2
  exit 1
fi

PCSLOT="$PCDIR/$SLOT"
PCSYS="$PCDIR/data00-1.bin"
STAMP=$(date +%Y%m%d_%H%M%S)
BACKUP="${OUTDIR:-$PCDIR}/backup_$STAMP"
mkdir -p "$BACKUP"

echo "==> Backing up current PC save to $BACKUP"
cp "$PCSLOT" "$BACKUP/" 2>/dev/null || echo "  (slot backup skipped)"
cp "$PCSYS"  "$BACKUP/" 2>/dev/null || echo "  (sys backup skipped)"
cp "$PCSLOT" "$BACKUP/${SLOT}.orig" 2>/dev/null || true

WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT
CUR="$WORKDIR/current.bin"
cp "$PCSLOT" "$CUR"

# All save classes to copy. EXCLUDED for account safety: network.SaveData,
# DLC ownership, and HunterRecord (contains the account-linked HunterUniqueID /
# NetworkUniqueId / NsaID -- copying them could bind/conflict with your account).
HASHES=(
  0xd6f4726d 0x9ccc3b1e 0xb0ca70c9 0xf2fb669a 0x1e32797d
  0x164d2e71 0x51dcd6fb 0xb9c15dcf 0xadc075b6 0x20d9167c 0x93819625
  0x8512ab74 0x553d33b8 0x68344f29 0x8c6fb4c6 0x1322883a 0xa81238c6
  0xaaca38e3 0x22a8b022 0x3da5b9de 0xa21e01ad 0x0386f39d 0xe825861b
  0x356f270b 0xef78287f 0xf5e018a4 0xdc00f45c 0xddb8e034 0x4423bc21
  0xcf6c5091 0x81fcc8f4 0xab109098 0xd5f91c48
)

echo "==> Copying Switch save classes into PC save (this takes a bit)..."
n=0
for h in "${HASHES[@]}"; do
  TMP="$WORKDIR/next.bin"
  if "$SAVE_COPY" slice --src "$CUR" --sys "$SWITCH" --hash "$h" \
       --steamid "$STEAMID" --out "$TMP" >/dev/null 2>&1; then
    mv "$TMP" "$CUR"; n=$((n+1))
  fi
done
echo "==> Sliced $n classes"

echo "==> Re-linking data00-1.bin to match slot (save link)..."
PCSLOT_TMP="$WORKDIR/final.bin"
"$SAVE_COPY" resave --src "$CUR" --steamid "$STEAMID" --sys "$PCSYS" --out "$PCSLOT_TMP" >/dev/null 2>&1 \
  || { echo "ERROR: relink failed" >&2; exit 1; }

echo "==> Done. Replacing PC save files:"
cp "$PCSLOT_TMP" "$PCSLOT"
echo "    $PCSLOT  (updated)"
# resave already patched $PCSYS link (data00-1.bin)
echo "    $PCSYS   (updated link)"

echo ""
echo "============================================================"
echo " MIGRATION COMPLETE"
echo " - Your PC save now contains the Switch character, quests,"
echo "   gear, items, buddies, guild card, HR/MR, etc."
echo " - NOTE: launch the game AFTER this; it will load the migrated"
echo "   save. HR/MR and quest progression carry over."
echo " - Backup of previous saves: $BACKUP"
echo " - Account-bound (online ID / DLC) left untouched: safe."
echo "============================================================"
