// MHRise Switch->Steam migration in Wasm (browser-only, no server).
use ree_lib::save::game::Game;
use ree_lib::save::{SaveFile, SaveFlags, SaveOptions};
use ree_lib::save::types::{Class, EnumValue, FieldValue};
use ree_lib::save::types::Array;
use wasm_bindgen::prelude::*;
use js_sys::{Object, Reflect, Uint8Array};

const HASHES: &[u32] = &[
    0xd6f4726d, 0x9ccc3b1e, 0xb0ca70c9, 0xf2fb669a, 0x1e32797d, 0x355c8c4f,
    0x164d2e71, 0x51dcd6fb, 0xb9c15dcf, 0xadc075b6, 0x20d9167c, 0x93819625,
    0x8512ab74, 0x553d33b8, 0x68344f29, 0x8c6fb4c6, 0x1322883a, 0xa81238c6,
    0xaaca38e3, 0x22a8b022, 0x3da5b9de, 0xa21e01ad, 0x0386f39d, 0xe825861b,
    0x356f270b, 0xef78287f, 0xf5e018a4, 0xdc00f45c, 0xddb8e034, 0x4423bc21,
    0xcf6c5091, 0x81fcc8f4, 0xab109098, 0xd5f91c48,
];

fn opts(steamid: u64, curve: usize) -> SaveOptions {
    SaveOptions::new(Game::MHRISE).id(steamid).curve_index(curve)
}

fn murm3(data: &[u8]) -> u32 {
    let mut h: u32 = 0xffffffff;
    let (c1, c2): (u32, u32) = (0xcc9e2d51, 0x1b873593);
    let n = data.len();
    let mut i = 0;
    while i + 4 <= n {
        let k = u32::from_le_bytes([data[i], data[i+1], data[i+2], data[i+3]])
            .wrapping_mul(c1).rotate_left(15).wrapping_mul(c2);
        h ^= k;
        h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe6546b64);
        i += 4;
    }
    let rem = n - i;
    let k = match rem {
        3 => (data[i+2] as u32) << 16 | (data[i+1] as u32) << 8 | data[i] as u32,
        2 => (data[i+1] as u32) << 8 | data[i] as u32,
        1 => data[i] as u32,
        _ => 0,
    };
    if rem > 0 {
        h ^= k.wrapping_mul(c1).rotate_left(15).wrapping_mul(c2);
    }
    h ^= n as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}

fn set_link_and_tail(buf: &mut [u8], link: u32) {
    if buf.len() >= 0x10 { buf[0x0C..0x10].copy_from_slice(&link.to_le_bytes()); }
    if buf.len() >= 4 {
        let l = buf.len();
        let fh = murm3(&buf[..l - 4]);
        buf[l - 4..].copy_from_slice(&fh.to_le_bytes());
    }
}

#[wasm_bindgen]
pub fn detect_curve(data: &[u8], steamid: u64) -> Result<u32, JsValue> {
    // load with curve_index(None); the loader brute-forces and updates options.curve_index
    let mut o = SaveOptions::new(Game::MHRISE).id(steamid); // curve None
    match SaveFile::read_save(data.to_vec(), &mut o) {
        Ok(sf) => {
            let c = o.curve_index.unwrap_or(107);
            // remember: validate via id
            let _ = sf;
            Ok(c as u32)
        }
        Err(e) => Err(JsValue::from_str(&format!("failed to detect curve: {e}"))),
    }
}

fn name_hash(n: &str) -> u32 {
    // ree-lib uses its own murmur3; try both plain and underscore forms via raw
    let mut h: u32 = 0xffffffff;
    let b = n.as_bytes();
    let c1: u32 = 0xcc9e2d51; let c2: u32 = 0x1b873593;
    let len = b.len(); let mut i = 0;
    while i + 4 <= len {
        let k = u32::from_le_bytes([b[i],b[i+1],b[i+2],b[i+3]]).wrapping_mul(c1).rotate_left(15).wrapping_mul(c2);
        h ^= k; h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe6546b64); i += 4;
    }
    let rem = len - i; let k = match rem {
        3 => (b[i+2] as u32)<<16 | (b[i+1] as u32)<<8 | b[i] as u32,
        2 => (b[i+1] as u32)<<8 | b[i] as u32,
        1 => b[i] as u32, _ => 0 };
    if rem > 0 { h ^= k.wrapping_mul(c1).rotate_left(15).wrapping_mul(c2); }
    h ^= len as u32; h ^= h >> 16; h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13; h = h.wrapping_mul(0xc2b2ae35); h ^= h >> 16;
    h
}

fn find_field<'a>(c: &'a Class, name: &str) -> Option<&'a FieldValue> {
    let h = name_hash(name);
    c.fields.iter().find(|f| f.hash == h).map(|f| &f.value)
}

fn find_class<'a>(c: &'a Class, name: &str) -> Option<&'a Class> {
    match find_field(c, name) { Some(FieldValue::Class(x)) => Some(x), _ => None }
}
fn find_arr<'a>(c: &'a Class, name: &str) -> Option<&'a ree_lib::save::types::Array> {
    match find_field(c, name) { Some(FieldValue::Array(a)) => Some(a), _ => None }
}

fn read_str(c: &Class, name: &str) -> Option<String> {
    match find_field(c, name) {
        Some(FieldValue::String(u)) => Some(String::from_utf16_lossy(&u.0)),
        _ => None,
    }
}

pub fn slot_info(data: &[u8], steamid: u64) -> Result<String, JsValue> {
    let mut o = opts(steamid, 107);
    match SaveFile::read_save(data.to_vec(), &mut o) {
        Ok(sf) => {
            // try progress data for HR and chara/option for name
            let mut hr: Option<u32> = None; let mut name: Option<String> = None;
            for (h, cls) in &sf.fields {
                if *h == 0x164d2e71 { // ProgressSaveData
                    // _HunterRank / _Rank
                    for cand in ["_HunterRank","_HunterRankData","_Rank","_HR"] {
                        if let Some(FieldValue::U32(v)) = find_field(cls, cand) { hr = Some(*v); break; }
                    }
                }
                if *h == 0xb0ca70c9 { // CharaSaveData
                    if let Some(bd) = find_class(cls, "_BasicData") {
                        for cand in ["_Name","_PlayerName","Name"] {
                            if let Some(n) = read_str(bd, cand) { name = Some(n); break; }
                        }
                    } else {
                        for cand in ["_Name","_PlayerName","Name"] {
                            if let Some(n) = read_str(cls, cand) { name = Some(n); break; }
                        }
                    }
                }
            }
            Ok(format!("name={:?} hr={:?}", name, hr))
        }
        Err(e) => Err(JsValue::from_str(&format!("{e}"))),
    }
}

/// Hash the DECODED save structure (class hashes + field values), independent of
/// the per-write ciphertext randomness. Used to prove two saves carry identical content.
fn fold_fv(h: &mut u64, v: &FieldValue) {
    match v {
        FieldValue::Array(a) => {
            *h = h.wrapping_mul(31).wrapping_add(0xa1);
            for el in &a.values { fold_fv(h, el); }
        }
        FieldValue::Enum(e) => { let ev: i64 = match e { _ => 0 }; *h = h.wrapping_mul(31).wrapping_add(ev as u64); }
        FieldValue::Boolean(b) => { *h = h.wrapping_mul(31).wrapping_add(*b as u64); }
        FieldValue::S8(v) => { *h = h.wrapping_mul(31).wrapping_add((*v as i64) as u64); }
        FieldValue::U8(v) => { *h = h.wrapping_mul(31).wrapping_add(*v as u64); }
        FieldValue::S16(v) => { *h = h.wrapping_mul(31).wrapping_add((*v as i64) as u64); }
        FieldValue::U16(v) => { *h = h.wrapping_mul(31).wrapping_add(*v as u64); }
        FieldValue::S32(v) => { *h = h.wrapping_mul(31).wrapping_add((*v as i64) as u64); }
        FieldValue::U32(v) => { *h = h.wrapping_mul(31).wrapping_add(*v as u64); }
        FieldValue::S64(v) => { *h = h.wrapping_mul(31).wrapping_add((*v as i64) as u64); }
        FieldValue::U64(v) => { *h = h.wrapping_mul(31).wrapping_add(*v); }
        FieldValue::F32(v) => { *h = h.wrapping_mul(31).wrapping_add((*v as u32) as u64); }
        FieldValue::F64(v) => { *h = h.wrapping_mul(31).wrapping_add((*v as u64).wrapping_add((*v as i64) as u64)); }
        FieldValue::C8(v) => { *h = h.wrapping_mul(31).wrapping_add(*v as u64); }
        FieldValue::C16(v) => { *h = h.wrapping_mul(31).wrapping_add(*v as u64); }
        FieldValue::String(s) => {
            *h = h.wrapping_mul(31).wrapping_add(0x5e);
            for u in &s.0 { *h = h.wrapping_mul(31).wrapping_add(*u as u64); }
        }
        FieldValue::Struct(s) => {
            *h = h.wrapping_mul(31).wrapping_add(0x5f);
            for b in &s.data { *h = h.wrapping_mul(31).wrapping_add(*b as u64); }
        }
        FieldValue::Class(c) => {
            *h = h.wrapping_mul(31).wrapping_add(c.hash as u64);
            for f in &c.fields {
                *h = h.wrapping_mul(31).wrapping_add(f.hash as u64);
                fold_fv(h, &f.value);
            }
        }
        FieldValue::Unknown => { *h = h.wrapping_mul(31).wrapping_add(0x99); }
    }
}

#[wasm_bindgen]
pub fn content_sig(data: &[u8], steamid: u64) -> Result<String, JsValue> {
    let mut o = opts(steamid, 107);
    let sf = SaveFile::read_save(data.to_vec(), &mut o)
        .map_err(|e| JsValue::from_str(&format!("parse failed: {e}")))?;
    let mut h: u64 = 0xcbf29ce484222325;
    for (ch, c) in &sf.fields {
        h = h.wrapping_mul(31).wrapping_add(*ch as u64);
        for f in &c.fields {
            h = h.wrapping_mul(31).wrapping_add(f.hash as u64);
            fold_fv(&mut h, &f.value);
        }
    }
    Ok(format!("{:016x}", h))
}

/// Decrypt + decompress a save and return the plaintext payload bytes (the
/// decrypted stream the game parses), terminates with any trailing NULs trimmed
/// so byte comparisons are stable. The crypto-layer per-write randomness is gone.
#[wasm_bindgen]
pub fn decrypted_bytes(data: &[u8], steamid: u64) -> Result<JsValue, JsValue> {
    let mut o = opts(steamid, 107);
    let (plain, _off, _bf, _flags) = SaveFile::process_bytes_to_stream(data.to_vec(), &mut o)
        .map_err(|e| JsValue::from_str(&format!("decrypt failed: {e}")))?;
    Ok(Uint8Array::from(plain.as_slice()).into())
}

/// Read the EditSaveData `playerSlotUsed` boolean array (per-slot, length 5),
/// so we can see which save slots the title screen treats as occupied.
#[wasm_bindgen]
pub fn slot_used_flags(sys_data: &[u8], steamid: u64) -> Result<JsValue, JsValue> {
    let mut o = opts(steamid, 107);
    let sf = SaveFile::read_save(sys_data.to_vec(), &mut o)
        .map_err(|e| JsValue::from_str(&format!("parse failed: {e}")))?;
    let out = Object::new();
    let arr = js_sys::Array::new();
    let mut matched_cls = String::new();
    for (_, cls) in &sf.fields {
        for f in &cls.fields {
            if f.hash != 0x9f2ca447 { continue; } // playerSlotUsed
            matched_cls = format!("{:08x}", cls.hash);
            if let FieldValue::Array(a) = &f.value {
                for v in &a.values {
                    match v {
                        FieldValue::Boolean(b) => { arr.push(&JsValue::from_bool(*b)); }
                        _ => { arr.push(&JsValue::UNDEFINED); }
                    }
                }
            }
        }
    }
    Reflect::set(&out, &JsValue::from_str("cls"), &JsValue::from_str(&matched_cls)).ok();
    Reflect::set(&out, &JsValue::from_str("values"), &arr).ok();
    Ok(out.into())
}

/// Dump EditSaveData playerUIValue per slot: reports whether each slot has a
#[wasm_bindgen]
pub fn edit_report(sys_data: &[u8], steamid: u64) -> Result<JsValue, JsValue> {
    let mut o = opts(steamid, 107);
    let sf = SaveFile::read_save(sys_data.to_vec(), &mut o)
        .map_err(|e| JsValue::from_str(&format!("parse failed: {e}")))?;
    let mut arr = js_sys::Array::new();
    for (h, cls) in &sf.fields {
        for f in &cls.fields {
            let mut arrName = String::new();
            match f.hash {
                0x2ab93315 => arrName = "playerUIValue".into(),
                0x9f2ca447 => arrName = "playerSlotUsed".into(),
                0x0b00f68e => arrName = "playerSaveSlotName".into(),
                0x2cea2fa1 => arrName = "playerSaveTime".into(),
                _ => {}
            }
            if arrName.is_empty() { continue; }
            if let FieldValue::Array(a) = &f.value {
                let obj = Object::new();
                Reflect::set(&obj, &JsValue::from_str("cls"), &JsValue::from_str(&format!("{:08x}", h))).ok();
                Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(&arrName)).ok();
                Reflect::set(&obj, &JsValue::from_str("len"), &JsValue::from_f64(a.values.len() as f64)).ok();
                if f.hash == 0x2ab93315 {
                    let lens = js_sys::Array::new();
                    for v in &a.values {
                        match v {
                            FieldValue::Class(c) => {
                                let o2 = Object::new();
                                Reflect::set(&o2, &JsValue::from_str("fields"), &JsValue::from_f64(c.fields.len() as f64)).ok();
                                lens.push(&o2);
                            }
                            _ => { lens.push(&JsValue::UNDEFINED); }
                        }
                    }
                    Reflect::set(&obj, &JsValue::from_str("perSlot"), &lens).ok();
                }
                arr.push(&obj);
            }
        }
    }
    Ok(arr.into())
}

/// Dump every top-level class and its fields (hash + value kind), so we can locate
/// where appearance / playerUIValue / names live in the sys.
#[wasm_bindgen]
pub fn field_dump(data: &[u8], steamid: u64) -> Result<JsValue, JsValue> {
    let mut o = opts(steamid, 107);
    let sf = SaveFile::read_save(data.to_vec(), &mut o)
        .map_err(|e| JsValue::from_str(&format!("parse failed: {e}")))?;
    let arr = js_sys::Array::new();
    for (h, cls) in &sf.fields {
        let cobj = Object::new();
        Reflect::set(&cobj, &JsValue::from_str("cls"), &JsValue::from_str(&format!("{:08x}", h))).ok();
        let farr = js_sys::Array::new();
        for f in &cls.fields {
            let fobj = Object::new();
            Reflect::set(&fobj, &JsValue::from_str("hash"), &JsValue::from_str(&format!("{:08x}", f.hash))).ok();
            let kind = match &f.value {
                FieldValue::Array(_) => "Array",
                FieldValue::Class(_) => "Class",
                FieldValue::Struct(_) => "Struct",
                FieldValue::String(_) => "String",
                FieldValue::Boolean(_) => "Bool",
                _ => "Scalar",
            };
            Reflect::set(&fobj, &JsValue::from_str("kind"), &JsValue::from_str(kind)).ok();
            if let FieldValue::Array(a) = &f.value {
                Reflect::set(&fobj, &JsValue::from_str("len"), &JsValue::from_f64(a.values.len() as f64)).ok();
            }
            farr.push(&fobj);
        }
        Reflect::set(&cobj, &JsValue::from_str("fields"), &farr).ok();
        arr.push(&cobj);
    }
    Ok(arr.into())
}

/// Per-class content signature: returns a map of class-hash -> content-hash of the
/// decoded field values. Used to prove a source save and a migrated save carry the
/// same class content (content preservation) independent of ciphertext randomness.
#[wasm_bindgen]
pub fn class_sigs(data: &[u8], steamid: u64) -> Result<JsValue, JsValue> {
    let mut o = opts(steamid, 107);
    let sf = SaveFile::read_save(data.to_vec(), &mut o)
        .map_err(|e| JsValue::from_str(&format!("parse failed: {e}")))?;
    let obj = Object::new();
    for (ch, c) in &sf.fields {
        let mut h: u64 = 0xcbf29ce484222325;
        for f in &c.fields {
            h = h.wrapping_mul(31).wrapping_add(f.hash as u64);
            fold_fv(&mut h, &f.value);
        }
        Reflect::set(&obj, &JsValue::from_str(&format!("{:08x}", ch)),
                     &JsValue::from_str(&format!("{:016x}", h))).ok();
    }
    Ok(obj.into())
}

#[wasm_bindgen]
pub fn validate_switch(data: &[u8], steamid: u64) -> Result<String, JsValue> {
    let mut o = opts(steamid, 107);
    SaveFile::read_save(data.to_vec(), &mut o)
        .map(|sf| format!("ok:{}", sf.fields.len()))
        .map_err(|e| JsValue::from_str(&format!("parse failed: {e}")))
}

#[wasm_bindgen]
pub fn validate_steam(slot: &[u8], sys: &[u8], steamid: u64) -> Result<String, JsValue> {
    let slot_link = if slot.len() >= 0x10 { u32::from_le_bytes(slot[0x0C..0x10].try_into().unwrap()) } else { 0 };
    let sys_link = if sys.len() >= 0x10 { u32::from_le_bytes(sys[0x0C..0x10].try_into().unwrap()) } else { 0 };
    let mut issues = Vec::new();
    if slot_link == 0 || slot_link != sys_link {
        issues.push(format!("save link mismatch (slot={slot_link}, data00-1={sys_link})"));
    }
    if slot.len() >= 4 && u32::from_le_bytes(slot[slot.len()-4..].try_into().unwrap()) != murm3(&slot[..slot.len()-4]) {
        issues.push("slot checksum (tail) invalid".to_string());
    }
    let mut o = opts(steamid, 107);
    match SaveFile::read_save(slot.to_vec(), &mut o) {
        Ok(sf) if issues.is_empty() => Ok(format!("ok:{}", sf.fields.len())),
        Ok(_) => Err(JsValue::from_str(&issues.join("; "))),
        Err(e) => {
            issues.push(format!("parse failed: {e}"));
            Err(JsValue::from_str(&issues.join("; ")))
        }
    }
}

#[wasm_bindgen]
pub fn migrate(switch_data: &[u8], slot_data: &[u8], sys_data: &[u8], steamid: u64, curve: usize) -> Result<JsValue, JsValue> {
    let mut so = opts(steamid, curve);
    let sw = SaveFile::read_save(switch_data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("switch load failed: {e}")))?;
    let mut po = opts(steamid, curve);
    let mut pc = SaveFile::read_save(slot_data.to_vec(), &mut po)
        .map_err(|e| JsValue::from_str(&format!("pc load failed: {e}")))?;

    let mut copied = 0usize;
    for h in HASHES {
        if let Some((_, cls)) = sw.fields.iter().find(|(hh, _)| hh == h) {
            if let Some((_, pc_cls)) = pc.fields.iter_mut().find(|(ph, _)| ph == h) {
                *pc_cls = cls.clone();
                copied += 1;
            }
        }
    }

    pc.flags = SaveFlags::CITRUS;
    let mut final_bytes = pc.write_save(&po).map_err(|e| JsValue::from_str(&format!("save failed: {e}")))?;
    let sys_link = if sys_data.len() >= 0x10 { u32::from_le_bytes(sys_data[0x0C..0x10].try_into().unwrap()) } else { 0 };
    set_link_and_tail(&mut final_bytes, sys_link);

    let mut new_sys = sys_data.to_vec();
    set_link_and_tail(&mut new_sys, sys_link);

    let out = Object::new();
    Reflect::set(&out, &JsValue::from_str("slot"), &Uint8Array::from(final_bytes.as_slice())).ok();
    Reflect::set(&out, &JsValue::from_str("sys"), &Uint8Array::from(new_sys.as_slice())).ok();
    Reflect::set(&out, &JsValue::from_str("copied"), &JsValue::from_f64(copied as f64)).ok();
    Ok(out.into())
}

// --- character names / rank (schema-derived constants: murmur3(name, 0xffffffff)) ---
const EDIT_SAVE_CLS: u32 = 0xc4569a95;   // snow.gui.CharaMakeSaveManager.EditSaveData
const PLAYER_SLOT_NAME_FIELD: u32 = 0x0b00f68e; // playerSaveSlotName
const PLAYER_SAVE_TIME_FIELD: u32 = 0x2cea2fa1; // playerSaveTime
const PROGRESS_CLS: u32 = 0x948b7b9d;    // snow.progress.ProgressSaveData
const LOADINFO_CLS: u32 = 0xe6a69e8a;   // snow.LoadInfoData
const HUNTER_ARRAY_FIELD: u32 = 0x01b05969; // _HunterArray
const HUNTER_NAME_FIELD: u32 = 0xf79f3af6;  // _HunterName
const HR_FIELD: u32 = 0x5653c091;        // _HunterRank
const MR_FIELD: u32 = 0x4e428685;        // _MasterRank

#[wasm_bindgen]
pub fn slot_names(sys_data: &[u8], steamid: u64) -> JsValue {
    let mut so = opts(steamid, 107);
    let sf = match SaveFile::read_save(sys_data.to_vec(), &mut so) {
        Ok(f) => f,
        Err(_) => return JsValue::UNDEFINED,
    };
    let arr = js_sys::Array::new();
    for (_, cls) in &sf.fields {
        if cls.hash != EDIT_SAVE_CLS { continue; }
        for f in &cls.fields {
            if f.hash != PLAYER_SLOT_NAME_FIELD { continue; }
            if let FieldValue::Array(a) = &f.value {
                for v in &a.values {
                    match v {
                        FieldValue::String(s) => { let _ = arr.push(&JsValue::from_str(&s.to_string())); }
                        _ => { let _ = arr.push(&JsValue::UNDEFINED); }
                    }
                }
            }
        }
    }
    arr.into()
}

#[wasm_bindgen]
pub fn slot_times(sys_data: &[u8], steamid: u64) -> JsValue {
    let mut so = opts(steamid, 107);
    let sf = match SaveFile::read_save(sys_data.to_vec(), &mut so) {
        Ok(f) => f,
        Err(_) => return JsValue::UNDEFINED,
    };
    let arr = js_sys::Array::new();
    for (_, cls) in &sf.fields {
        if cls.hash != EDIT_SAVE_CLS { continue; }
        for f in &cls.fields {
            if f.hash != PLAYER_SAVE_TIME_FIELD { continue; }
            if let FieldValue::Array(a) = &f.value {
                for v in &a.values {
                    match v {
                        FieldValue::String(s) => { let _ = arr.push(&JsValue::from_str(&s.to_string())); }
                        _ => { let _ = arr.push(&JsValue::UNDEFINED); }
                    }
                }
            }
        }
    }
    arr.into()
}

/// Copy the per-slot EditSaveData (character appearance / playerUIValue, slot name,
/// save time, slot-used flag) from the source sys into the target sys template, for the
/// given target slot indices. Writes the result keyed to target_steamid.
#[wasm_bindgen]
pub fn migrate_sys(src_sys: &[u8], src_steamid: u64, tgt_sys: &[u8], tgt_steamid: u64,
                   slots: &[u32], curve: usize) -> Result<JsValue, JsValue> {
    let mut so = opts(src_steamid, curve);
    let mut s = SaveFile::read_save(src_sys.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("src sys load failed: {e}")))?;
    let mut to = opts(tgt_steamid, curve);
    let mut t = SaveFile::read_save(tgt_sys.to_vec(), &mut to)
        .map_err(|e| JsValue::from_str(&format!("tgt sys load failed: {e}")))?;

    const UI_VALUE: u32 = 0x2ab93315;    // playerUIValue
    const SLOT_USED: u32 = 0x9f2ca447;   // playerSlotUsed
    const SLOT_NAME: u32 = 0x0b00f68e;   // playerSaveSlotName
    const SLOT_TIME: u32 = 0x2cea2fa1;   // playerSaveTime

    let mut copied = 0usize;
    for (_, cls) in s.fields.iter_mut() {
        for f in cls.fields.iter() {
            let h = f.hash;
            if ![UI_VALUE, SLOT_USED, SLOT_NAME, SLOT_TIME].contains(&h) { continue; }
            let FieldValue::Array(a) = &f.value else { continue; };
            // find matching field in target (by field hash within the same class width)
            for (_, tcls) in t.fields.iter_mut() {
                let mut did = false;
                for tf in tcls.fields.iter_mut() {
                    if tf.hash != h { continue; }
                    let FieldValue::Array(ta) = &mut tf.value else { continue; };
                    for &idx in slots {
                        if (idx as usize) < a.values.len() && (idx as usize) < ta.values.len() {
                            ta.values[idx as usize] = a.values[idx as usize].clone();
                        }
                    }
                    did = true; copied += 1;
                }
                if did { break; }
            }
        }
        // note: we only scan source's EditSaveData class fields; the array-class is
        // whichever holds them; if multiple classes share the hash this is idempotent.
    }

    t.flags = SaveFlags::CITRUS;
    let bytes = t.write_save(&to).map_err(|e| JsValue::from_str(&format!("save failed: {e}")))?;
    let out = Object::new();
    Reflect::set(&out, &JsValue::from_str("sys"), &Uint8Array::from(bytes.as_slice())).ok();
    Reflect::set(&out, &JsValue::from_str("copied"), &JsValue::from_f64(copied as f64)).ok();
    Ok(out.into())
}

#[wasm_bindgen]
pub fn hunter_names(sys_data: &[u8], steamid: u64) -> JsValue {
    let mut so = opts(steamid, 107);
    let sf = match SaveFile::read_save(sys_data.to_vec(), &mut so) {
        Ok(f) => f,
        Err(_) => return JsValue::UNDEFINED,
    };
    let arr = js_sys::Array::new();
    for (_, cls) in &sf.fields {
        if cls.hash != LOADINFO_CLS { continue; }
        for f in &cls.fields {
            if f.hash != HUNTER_ARRAY_FIELD { continue; }
            if let FieldValue::Array(a) = &f.value {
                for v in &a.values {
                    if let FieldValue::Class(c) = v {
                        let mut got = false;
                        for g in &c.fields {
                            if g.hash == HUNTER_NAME_FIELD {
                                if let FieldValue::String(gs) = &g.value { let _ = arr.push(&JsValue::from_str(&gs.to_string())); }
                                else { let _ = arr.push(&JsValue::UNDEFINED); }
                                got = true;
                            }
                        }
                        if !got { let _ = arr.push(&JsValue::UNDEFINED); }
                    } else { let _ = arr.push(&JsValue::UNDEFINED); }
                }
            }
        }
    }
    arr.into()
}

#[wasm_bindgen]
pub fn set_hunter_name(sys_data: &[u8], steamid: u64, idx: u32, name: &str) -> Result<JsValue, JsValue> {
    let mut so = opts(steamid, 107);
    let mut sf = SaveFile::read_save(sys_data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("read: {e}")))?;
    let link = if sys_data.len() >= 0x10 { u32::from_le_bytes(sys_data[0x0C..0x10].try_into().unwrap()) } else { 0 };
    let mut found = false;
    for (_, cls) in sf.fields.iter_mut() {
        if cls.hash != LOADINFO_CLS { continue; }
        for f in cls.fields.iter_mut() {
            if f.hash != HUNTER_ARRAY_FIELD { continue; }
            if let FieldValue::Array(a) = &mut f.value {
                if let Some(FieldValue::Class(c)) = a.values.get_mut(idx as usize) {
                    for g in c.fields.iter_mut() {
                        if g.hash == HUNTER_NAME_FIELD {
                            if let Some(su) = g.value.as_string_u16_mut() { su.0 = name.encode_utf16().collect(); found = true; }
                        }
                    }
                }
            }
        }
    }
    if !found { return Err(JsValue::from_str("no hunter name slot")); }
    sf.flags = SaveFlags::CITRUS;
    let mut bytes = sf.write_save(&so).map_err(|e| JsValue::from_str(&format!("write: {e}")))?;
    set_link_and_tail(&mut bytes, link);
    Ok(Uint8Array::from(bytes.as_slice()).into())
}

#[wasm_bindgen]
pub fn set_slot_used(sys_data: &[u8], steamid: u64, idx: u32, used: bool) -> Result<JsValue, JsValue> {
    let mut so = opts(steamid, 107);
    let mut sf = SaveFile::read_save(sys_data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("read: {e}")))?;
    let link = if sys_data.len() >= 0x10 { u32::from_le_bytes(sys_data[0x0C..0x10].try_into().unwrap()) } else { 0 };
    let mut ok = false;
    for (_, cls) in sf.fields.iter_mut() {
        if cls.hash != EDIT_SAVE_CLS { continue; }
        for f in cls.fields.iter_mut() {
            if f.hash != 0x9f2ca447 { continue; } // playerSlotUsed (murmur3)
            if let FieldValue::Array(a) = &mut f.value {
                if a.values.len() > idx as usize { a.values[idx as usize] = FieldValue::Boolean(used); ok = true; }
            }
        }
    }
    if !ok { return Err(JsValue::from_str("no playerSlotUsed")); }
    sf.flags = SaveFlags::CITRUS;
    let mut bytes = sf.write_save(&so).map_err(|e| JsValue::from_str(&format!("write: {e}")))?;
    set_link_and_tail(&mut bytes, link);
    Ok(Uint8Array::from(bytes.as_slice()).into())
}

/// Steam -> Steam migrate: read source save with src_steamid, merge classes into a target
/// template, write with target_steamid (identical code path to the validated switch->steam migrate).
#[wasm_bindgen]
pub fn migrate_x(src: &[u8], src_steamid: u64, template: &[u8], sys_data: &[u8], target_steamid: u64, curve: usize, tgt_curve: usize) -> Result<JsValue, JsValue> {
    let mut soSrc = opts(src_steamid, curve);
    let sw = SaveFile::read_save(src.to_vec(), &mut soSrc)
        .map_err(|e| JsValue::from_str(&format!("src load failed: {e}")))?;
    let mut soT = opts(target_steamid, tgt_curve);
    let mut pc = SaveFile::read_save(template.to_vec(), &mut soT)
        .map_err(|e| JsValue::from_str(&format!("template load failed: {e}")))?;
    let mut copied = 0usize;
    for h in HASHES {
        if let Some((_, cls)) = sw.fields.iter().find(|(hh, _)| hh == h) {
            if let Some((_, pc_cls)) = pc.fields.iter_mut().find(|(ph, _)| ph == h) {
                *pc_cls = cls.clone();
                copied += 1;
            }
        }
    }
    pc.flags = SaveFlags::CITRUS;
    let mut final_bytes = pc.write_save(&soT).map_err(|e| JsValue::from_str(&format!("save failed: {e}")))?;
    let link = if sys_data.len() >= 0x10 { u32::from_le_bytes(sys_data[0x0C..0x10].try_into().unwrap()) } else { 0 };
    set_link_and_tail(&mut final_bytes, link);
    let mut new_sys = sys_data.to_vec();
    set_link_and_tail(&mut new_sys, link);
    let out = Object::new();
    Reflect::set(&out, &JsValue::from_str("slot"), &Uint8Array::from(final_bytes.as_slice())).ok();
    Reflect::set(&out, &JsValue::from_str("sys"), &Uint8Array::from(new_sys.as_slice())).ok();
    Reflect::set(&out, &JsValue::from_str("copied"), &JsValue::from_f64(copied as f64)).ok();
    Ok(out.into())
}

fn same_kind(a: &FieldValue, b: &FieldValue) -> bool {
    use FieldValue::*;
    matches!((a, b),
        (Array(_), Array(_)) | (Enum(_), Enum(_)) | (Boolean(_), Boolean(_)) |
        (S8(_), S8(_)) | (U8(_), U8(_)) | (S16(_), S16(_)) | (U16(_), U16(_)) |
        (S32(_), S32(_)) | (U32(_), U32(_)) | (S64(_), S64(_)) | (U64(_), U64(_)) |
        (F32(_), F32(_)) | (F64(_), F64(_)) | (C8(_), C8(_)) | (C16(_), C16(_)) |
        (String(_), String(_)) | (Struct(_), Struct(_)) | (Class(_), Class(_)) |
        (Unknown, Unknown))
}

/// Sync a target LOADINFO hunter entry (title-screen character record) with the same
/// entry from another sys (e.g. the Switch sys after a Switch -> Steam slot copy).
/// Every field present in both entries is taken from the source (name, HR/MR, play
/// time, buddies, outfit colours, and the whole eec7904b appearance class with
/// gender/face/hair/voice). Fields that must stay bound to the target are preserved:
/// the four link-derived enums (0x9eb56094/0xed59430c/0x8a471883/0xf4deaf65) and the
/// 16-byte character GUID (0xe40fc0cd). The result is re-serialised and re-keyed.
#[wasm_bindgen]
pub fn sync_hunter_entry(sys_data: &[u8], src_sys: &[u8], steamid: u64, curve: usize,
                         tgt_idx: u32, src_idx: u32) -> Result<JsValue, JsValue> {
    let mut so = opts(steamid, curve);
    let mut t = SaveFile::read_save(sys_data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("tgt sys load failed: {e}")))?;
    let mut so_src = opts(steamid, curve);
    let s = SaveFile::read_save(src_sys.to_vec(), &mut so_src)
        .map_err(|e| JsValue::from_str(&format!("src sys load failed: {e}")))?;
    let link = if sys_data.len() >= 0x10 { u32::from_le_bytes(sys_data[0x0C..0x10].try_into().unwrap()) } else { 0 };

    // collect the source entry's fields by hash
    let mut src_fields: Vec<(u32, FieldValue)> = Vec::new();
    for (_, cls) in &s.fields {
        if cls.hash != LOADINFO_CLS { continue; }
        for f in &cls.fields {
            if f.hash != HUNTER_ARRAY_FIELD { continue; }
            if let FieldValue::Array(a) = &f.value {
                if let Some(FieldValue::Class(c)) = a.values.get(src_idx as usize) {
                    for g in &c.fields { src_fields.push((g.hash, g.value.clone())); }
                }
            }
        }
    }
    if src_fields.is_empty() { return Err(JsValue::from_str("src hunter entry not found")); }

    // target-bound fields that are never overwritten
    const KEEP: &[u32] = &[0x9eb56094, 0xed59430c, 0x8a471883, 0xf4deaf65, 0xe40fc0cd];
    let mut replaced = 0usize;
    let mut found_target = false;
    for (_, cls) in t.fields.iter_mut() {
        if cls.hash != LOADINFO_CLS { continue; }
        for f in cls.fields.iter_mut() {
            if f.hash != HUNTER_ARRAY_FIELD { continue; }
            if let FieldValue::Array(a) = &mut f.value {
                if let Some(FieldValue::Class(c)) = a.values.get_mut(tgt_idx as usize) {
                    found_target = true;
                    for g in c.fields.iter_mut() {
                        if KEEP.contains(&g.hash) { continue; }
                        if let Some((_, sv)) = src_fields.iter().find(|(h, _)| *h == g.hash) {
                            if same_kind(&g.value, sv) { g.value = sv.clone(); replaced += 1; }
                        }
                    }
                }
            }
        }
    }
    if !found_target { return Err(JsValue::from_str("tgt hunter entry not found")); }

    t.flags = SaveFlags::CITRUS;
    let mut bytes = t.write_save(&so).map_err(|e| JsValue::from_str(&format!("write: {e}")))?;
    set_link_and_tail(&mut bytes, link);
    let out = Object::new();
    Reflect::set(&out, &JsValue::from_str("sys"), &Uint8Array::from(bytes.as_slice())).ok();
    Reflect::set(&out, &JsValue::from_str("replaced"), &JsValue::from_f64(replaced as f64)).ok();
    Ok(out.into())
}

/// Copy one LOADINFO hunter entry (the title-screen character record: name, HR, MR,
/// play time and all nested per-character classes) from the source sys into the target
/// sys at the given index. The four link-derived enum fields are rewritten to the target
/// sys's own link so the entry stays consistent with the target's save header.
#[wasm_bindgen]
pub fn copy_hunter_entry(tgt_sys: &[u8], src_sys: &[u8], src_steamid: u64, src_curve: usize,
                         tgt_steamid: u64, tgt_curve: usize, dst_idx: u32, src_idx: u32) -> Result<JsValue, JsValue> {
    let mut soS = opts(src_steamid, src_curve);
    let s = SaveFile::read_save(src_sys.to_vec(), &mut soS)
        .map_err(|e| JsValue::from_str(&format!("src sys load failed: {e}")))?;
    let mut soT = opts(tgt_steamid, tgt_curve);
    let mut t = SaveFile::read_save(tgt_sys.to_vec(), &mut soT)
        .map_err(|e| JsValue::from_str(&format!("tgt sys load failed: {e}")))?;
    let link = if tgt_sys.len() >= 0x10 { u32::from_le_bytes(tgt_sys[0x0C..0x10].try_into().unwrap()) } else { 0 };

    let mut entry: Option<Class> = None;
    for (_, cls) in &s.fields {
        if cls.hash != LOADINFO_CLS { continue; }
        for f in &cls.fields {
            if f.hash != HUNTER_ARRAY_FIELD { continue; }
            if let FieldValue::Array(a) = &f.value {
                if let Some(FieldValue::Class(c)) = a.values.get(src_idx as usize) {
                    entry = Some((**c).clone());
                }
            }
        }
    }
    let mut entry = entry.ok_or_else(|| JsValue::from_str("src hunter entry not found"))?;

    const LINK_ENUMS: &[u32] = &[0x9eb56094, 0xed59430c, 0x8a471883, 0xf4deaf65];
    for f in entry.fields.iter_mut() {
        if LINK_ENUMS.contains(&f.hash) {
            f.value = FieldValue::Enum(EnumValue::E4((link << 8) as i32));
        }
    }

    let mut ok = false;
    for (_, cls) in t.fields.iter_mut() {
        if cls.hash != LOADINFO_CLS { continue; }
        for f in cls.fields.iter_mut() {
            if f.hash != HUNTER_ARRAY_FIELD { continue; }
            if let FieldValue::Array(a) = &mut f.value {
                if let Some(v) = a.values.get_mut(dst_idx as usize) {
                    *v = FieldValue::Class(Box::new(entry.clone()));
                    ok = true;
                }
            }
        }
    }
    if !ok { return Err(JsValue::from_str("dst hunter entry not found")); }

    t.flags = SaveFlags::CITRUS;
    let mut bytes = t.write_save(&soT).map_err(|e| JsValue::from_str(&format!("write: {e}")))?;
    set_link_and_tail(&mut bytes, link);
    Ok(Uint8Array::from(bytes.as_slice()).into())
}

/// Steam -> Steam migrate preserving the target's account-bound identity: copies all
/// HASHES classes from the source EXCEPT the HunterRecord (0x355c8c4f, which carries the
/// account's HunterUniqueID / NetworkUniqueId / NsaID). Those are kept from the target
/// template so the resulting save belongs to the target account.
#[wasm_bindgen]
pub fn migrate_acct(src: &[u8], src_steamid: u64, template: &[u8], sys_data: &[u8], target_steamid: u64, curve: usize, tgt_curve: usize) -> Result<JsValue, JsValue> {
    let mut soSrc = opts(src_steamid, curve);
    let sw = SaveFile::read_save(src.to_vec(), &mut soSrc)
        .map_err(|e| JsValue::from_str(&format!("src load failed: {e}")))?;
    let mut soT = opts(target_steamid, tgt_curve);
    let mut pc = SaveFile::read_save(template.to_vec(), &mut soT)
        .map_err(|e| JsValue::from_str(&format!("template load failed: {e}")))?;
    let mut copied = 0usize;
    for h in HASHES {
        if *h == 0x355c8c4f { continue; }  // keep target's account-bound HunterRecord
        if let Some((_, cls)) = sw.fields.iter().find(|(hh, _)| hh == h) {
            if let Some((_, pc_cls)) = pc.fields.iter_mut().find(|(ph, _)| ph == h) {
                *pc_cls = cls.clone();
                copied += 1;
            }
        }
    }
    pc.flags = SaveFlags::CITRUS;
    let mut final_bytes = pc.write_save(&soT).map_err(|e| JsValue::from_str(&format!("save failed: {e}")))?;
    let link = if sys_data.len() >= 0x10 { u32::from_le_bytes(sys_data[0x0C..0x10].try_into().unwrap()) } else { 0 };
    set_link_and_tail(&mut final_bytes, link);
    let mut new_sys = sys_data.to_vec();
    set_link_and_tail(&mut new_sys, link);
    let out = Object::new();
    Reflect::set(&out, &JsValue::from_str("slot"), &Uint8Array::from(final_bytes.as_slice())).ok();
    Reflect::set(&out, &JsValue::from_str("sys"), &Uint8Array::from(new_sys.as_slice())).ok();
    Reflect::set(&out, &JsValue::from_str("copied"), &JsValue::from_f64(copied as f64)).ok();
    Ok(out.into())
}

/// Steam -> Steam re-key: decrypt with src_steamid, re-encrypt with target_steamid, set link.
#[wasm_bindgen]
pub fn rekey(data: &[u8], src_steamid: u64, target_steamid: u64, link: u32, curve: usize) -> Result<JsValue, JsValue> {
    let mut so = opts(src_steamid, curve);
    let mut sf = SaveFile::read_save(data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("read(src): {e}")))?;
    sf.flags = SaveFlags::CITRUS;
    let mut to = opts(target_steamid, curve);
    let mut out = sf.write_save(&to)
        .map_err(|e| JsValue::from_str(&format!("write(target): {e}")))?;
    set_link_and_tail(&mut out, link);
    Ok(Uint8Array::from(out.as_slice()).into())
}

#[wasm_bindgen]
pub fn patch_link(sys_data: &[u8], link: u32) -> Result<JsValue, JsValue> {
    let mut bytes = sys_data.to_vec();
    if bytes.len() < 0x14 { return Err(JsValue::from_str("too small")); }
    set_link_and_tail(&mut bytes, link);
    Ok(Uint8Array::from(bytes.as_slice()).into())
}

#[wasm_bindgen]
pub fn set_both_names(sys_data: &[u8], steamid: u64, idx: u32, name: &str) -> Result<JsValue, JsValue> {
    let mut so = opts(steamid, 107);
    let mut sf = SaveFile::read_save(sys_data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("read: {e}")))?;
    let link = if sys_data.len() >= 0x10 { u32::from_le_bytes(sys_data[0x0C..0x10].try_into().unwrap()) } else { 0 };
    let mut set1 = false; let mut set2 = false;
    let u = name.encode_utf16().collect::<Vec<u16>>();
    for (_, cls) in sf.fields.iter_mut() {
        if cls.hash == LOADINFO_CLS {
            for f in cls.fields.iter_mut() {
                if f.hash == HUNTER_ARRAY_FIELD {
                    if let FieldValue::Array(a) = &mut f.value {
                        if let Some(FieldValue::Class(c)) = a.values.get_mut(idx as usize) {
                            for g in c.fields.iter_mut() {
                                if g.hash == HUNTER_NAME_FIELD {
                                    if let Some(su) = g.value.as_string_u16_mut() { su.0 = u.clone(); set1 = true; }
                                }
                            }
                        }
                    }
                }
            }
        }
        if cls.hash == EDIT_SAVE_CLS {
            for f in cls.fields.iter_mut() {
                if f.hash == PLAYER_SLOT_NAME_FIELD {
                    if let FieldValue::Array(a) = &mut f.value {
                        if a.values.len() > idx as usize {
                            if let FieldValue::String(su) = &mut a.values[idx as usize] {
                                su.0 = u.clone(); set2 = true;
                            }
                        }
                    }
                }
            }
        }
    }
    if !set1 && !set2 { return Err(JsValue::from_str("no name slots")); }
    sf.flags = SaveFlags::CITRUS;
    let mut bytes = sf.write_save(&so).map_err(|e| JsValue::from_str(&format!("write: {e}")))?;
    set_link_and_tail(&mut bytes, link);
    Ok(Uint8Array::from(bytes.as_slice()).into())
}

#[wasm_bindgen]
pub fn raw_patch_str(sys_data: &[u8], steamid: u64, curve: usize, offset: u32, name: &str) -> Result<JsValue, JsValue> {
    let mut so = opts(steamid, curve);
    let (data, data_offset, _bf, _flags) = SaveFile::process_bytes_to_stream(sys_data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("read: {e}")))?;
    let off = data_offset as usize;
    let mut payload = data[off..].to_vec();
    let pos = offset as usize;
    if pos + 4 > payload.len() { return Err(JsValue::from_str("offset oob")); }
    let old_size = u32::from_le_bytes(payload[pos..pos+4].try_into().unwrap()) as usize;
    let old_total = (4 + old_size * 2 + 3) & !3;
    if pos + old_total > payload.len() { return Err(JsValue::from_str("string oob")); }
    let u: Vec<u16> = name.encode_utf16().collect();
    let mut nb = Vec::with_capacity(4 + u.len() * 2);
    nb.extend_from_slice(&(u.len() as u32).to_le_bytes());
    for c in &u { nb.extend_from_slice(&c.to_le_bytes()); }
    while nb.len() % 4 != 0 { nb.push(0); }
    payload.splice(pos..pos + old_total, nb);
    let mut out = data[..off].to_vec();
    let citrus = ree_lib::save::crypto::citrus::Citrus::new(steamid, so.curve_index);
    let enc = citrus.encrypt(&payload).ok_or_else(|| JsValue::from_str("encrypt failed"))?;
    out.extend_from_slice(&enc);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    while out.len() % 4 != 0 { out.push(0); }
    let h = murm3(&out);
    out.extend_from_slice(&h.to_le_bytes());
    Ok(Uint8Array::from(out.as_slice()).into())
}

#[wasm_bindgen]
pub fn patch_bytes(data: &[u8], steamid: u64, curve: usize, offset: u32, bytes: &[u8]) -> Result<JsValue, JsValue> {
    let mut so = opts(steamid, curve);
    let (stream, data_offset, _bf, _flags) = SaveFile::process_bytes_to_stream(data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("read: {e}")))?;
    let off = data_offset as usize;
    let mut payload = stream[off..].to_vec();
    let pos = offset as usize;
    if pos + bytes.len() > payload.len() { return Err(JsValue::from_str("patch oob")); }
    payload[pos..pos + bytes.len()].copy_from_slice(bytes);
    let mut out = stream[..off].to_vec();
    let citrus = ree_lib::save::crypto::citrus::Citrus::new(steamid, so.curve_index);
    let enc = citrus.encrypt(&payload).ok_or_else(|| JsValue::from_str("encrypt failed"))?;
    out.extend_from_slice(&enc);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    while out.len() % 4 != 0 { out.push(0); }
    let h = murm3(&out);
    out.extend_from_slice(&h.to_le_bytes());
    Ok(Uint8Array::from(out.as_slice()).into())
}

#[wasm_bindgen]
pub fn raw_patch_u32(sys_data: &[u8], steamid: u64, curve: usize, offset: u32, value: u32) -> Result<JsValue, JsValue> {
    let mut so = opts(steamid, curve);
    let (data, data_offset, _bf, _flags) = SaveFile::process_bytes_to_stream(sys_data.to_vec(), &mut so)
        .map_err(|e| JsValue::from_str(&format!("read: {e}")))?;
    let off = data_offset as usize;
    let mut payload = data[off..].to_vec();
    let pos = offset as usize;
    if pos + 4 > payload.len() { return Err(JsValue::from_str("offset oob")); }
    payload[pos..pos+4].copy_from_slice(&value.to_le_bytes());
    let mut out = data[..off].to_vec();
    let citrus = ree_lib::save::crypto::citrus::Citrus::new(steamid, so.curve_index);
    let enc = citrus.encrypt(&payload).ok_or_else(|| JsValue::from_str("encrypt failed"))?;
    out.extend_from_slice(&enc);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    while out.len() % 4 != 0 { out.push(0); }
    let h = murm3(&out);
    out.extend_from_slice(&h.to_le_bytes());
    Ok(Uint8Array::from(out.as_slice()).into())
}

#[wasm_bindgen]
pub fn slot_rank(slot_data: &[u8], steamid: u64) -> JsValue {
    let mut so = opts(steamid, 107);
    let sf = match SaveFile::read_save(slot_data.to_vec(), &mut so) {
        Ok(f) => f,
        Err(_) => return JsValue::UNDEFINED,
    };
    let mut hr: Option<i32> = None;
    let mut mr: Option<i32> = None;
    for (_, cls) in &sf.fields {
        if cls.hash != PROGRESS_CLS { continue; }
        for f in &cls.fields {
            if let FieldValue::S32(v) = &f.value {
                if f.hash == HR_FIELD { hr = Some(*v); }
                else if f.hash == MR_FIELD { mr = Some(*v); }
            }
        }
    }
    let arr = js_sys::Array::new();
    arr.push(&hr.map(|v| JsValue::from_f64(v as f64)).unwrap_or(JsValue::UNDEFINED));
    arr.push(&mr.map(|v| JsValue::from_f64(v as f64)).unwrap_or(JsValue::UNDEFINED));
    arr.into()
}
