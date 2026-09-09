#!/usr/bin/env bash
# Tests for mhrise-save-migrate. Run: cd tests && ./run_tests.sh
# Self-checks: binary presence, crypto header/link/murmur invariants, slice
# behavior, and (if fixtures exist) a full migration + verification.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"
BIN="$ROOT/save_copy"
FIX="$ROOT/tests/fixtures"
FAIL=0

say()  { printf "\n== %s ==\n" "$*"; }
ok()   { printf "  PASS  %s\n" "$*"; }
fail() { printf "  FAIL  %s\n" "$*"; FAIL=1; }

say "1. binary + assets presence"
[ -x "$BIN" ] && ok "save_copy executable" || fail "save_copy missing"
[ -f "$ROOT/assets/mhrise/enumsmhrise.json" ] && ok "enums asset" || fail "assets/mhrise/enumsmhrise.json missing"

say "2. link/murmur round-trip (resave preserves link + valid tail)"
if [ -f "$FIX/pc_data002Slot.bin" ] && [ -f "$FIX/pc_data00-1.bin" ]; then
  T=$(mktemp -d)
  cp "$FIX/pc_data002Slot.bin" "$T/slot.bin"
  cp "$FIX/pc_data00-1.bin" "$T/sys.bin"
  "$BIN" resave --src "$T/slot.bin" --steamid 76561198180024509 --sys "$T/sys.bin" --out "$T/out.bin" >/dev/null 2>&1 \
    && ok "resave ran" || fail "resave failed"
  python3 - "$T" <<'PYEOF'
import sys, os
d=os.path.join(sys.argv[1],'out.bin'); s=os.path.join(sys.argv[1],'sys.bin')
slot=open(d,'rb').read(); sysd=open(s,'rb').read()
def m3(data, seed=0xffffffff):
    h=seed
    for off in range(0,len(data)//4*4,4):
        k=int.from_bytes(data[off:off+4],'little'); k=(k*0xcc9e2d51)&0xffffffff
        k=((k<<15)|(k>>17))&0xffffffff; k=(k*0x1b873593)&0xffffffff
        h^=k; h=((h<<13)|(h>>19))&0xffffffff; h=(h*5+0xe6546b64)&0xffffffff
    tail=data[len(data)//4*4:]; k=0
    if len(tail)>=3: k^=tail[2]<<16
    if len(tail)>=2: k^=tail[1]<<8
    if len(tail)>=1:
        k^=tail[0]; k=(k*0xcc9e2d51)&0xffffffff; k=((k<<15)|(k>>17))&0xffffffff
        k=(k*0x1b873593)&0xffffffff; h^=k
    h^=len(data); h^=h>>16; h=(h*0x85ebca6b)&0xffffffff; h^=h>>13
    h=(h*0xc2b2ae35)&0xffffffff; h^=h>>16; return h
lk_s=int.from_bytes(slot[0x0C:0x10],'little'); lk_y=int.from_bytes(sysd[0x0C:0x10],'little')
ok1 = (lk_s==lk_y) and (int.from_bytes(slot[-4:],'little')==m3(slot[:-4]))
print(("  PASS  link match(%d==%d)+murmur" % (lk_s,lk_y)) if ok1 else "  FAIL  link/murmur")
sys.exit(0 if ok1 else 1)
PYEOF
  [ $? -eq 0 ] || FAIL=1
  rm -rf "$T"
else
  echo "  (fixtures absent - skipping round-trip)"
fi

say "3. slice behavior (copy one class from switch->pc, verify reload)"
if [ -f "$FIX/switch_data001Slot.bin" ] && [ -f "$FIX/pc_data002Slot.bin" ]; then
  T=$(mktemp -d)
  cp "$FIX/pc_data002Slot.bin" "$T/pc.bin"
  OUT=$("$BIN" slice --src "$T/pc.bin" --sys "$FIX/switch_data001Slot.bin" --hash 0xd6f4726d --steamid 76561198180024509 --out "$T/out.bin" 2>&1)
  echo "$OUT" | grep -q "copied 1 top-level" && ok "sliced one class" || fail "slice failed: $OUT"
  rm -rf "$T"
else
  echo "  (fixtures absent - skipping slice)"
fi

say "4. full migration script (if fixtures present)"
if [ -f "$FIX/switch_data001Slot.bin" ] && [ -f "$FIX/pc_data002Slot.bin" ]; then
  T=$(mktemp -d); mkdir -p "$T/save"
  cp "$FIX/pc_data002Slot.bin" "$T/save/data001Slot.bin"
  cp "$FIX/pc_data00-1.bin" "$T/save/data00-1.bin"
  if (cd "$ROOT" && ./migrate_switch_to_steam.sh --switch "$FIX/switch_data001Slot.bin" --steamid64 76561198180024509 --pc-dir "$T/save" >/dev/null 2>&1); then
    ok "script completed"
    python3 - "$T/save/data001Slot.bin" "$FIX/switch_data001Slot.bin" <<'PYEOF' || FAIL=1
import sys
slot=open(sys.argv[1],'rb').read()
lk=int.from_bytes(slot[0x0C:0x10],'little')
print("  PASS  migrated slot present (link=%d)" % lk) if lk else print("  FAIL  bad link")
PYEOF
  else
    fail "migration script failed"
  fi
  rm -rf "$T"
else
  echo "  (fixtures absent - skipping full migration)"
fi

say "5. hunter-entry sync mapping (cross-slot, wasm)"
if command -v node >/dev/null 2>&1 && [ -f "$ROOT/wasm/pkgnode/mhsave_wasm.js" ] && \
   [ -f "$ROOT/web/e2e/fixtures/switch_data00-1.bin" ]; then
  if node "$ROOT/tests/sync_mapping_test.js"; then ok "sync mapping"; else fail "sync mapping"; fi
else
  echo "  (node or wasm/fixtures absent - skipping sync mapping)"
fi

say "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL PASSED"; else echo "SOME FAILED"; exit 1; fi
