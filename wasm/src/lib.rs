// MHRise Switch->Steam migration in Wasm (browser-only, no server).
use ree_lib::save::game::Game;
use ree_lib::save::{SaveFile, SaveFlags, SaveOptions};
use ree_lib::save::types::{Class, FieldValue};
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
