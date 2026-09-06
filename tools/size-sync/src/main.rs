use ree_lib::save::game::Game;
use ree_lib::save::{SaveFile, SaveFlags, SaveOptions};
use ree_lib::save::types::{Class, FieldValue};

const HUNTER_RECORD: u32 = 0x355c8c4f; // snow.HunterRecordManager.HunterReocrdSaveData
const ENEMY_SIZE_MIN: u32 = 0xa23ee890; // EnemySizeMin
const ENEMY_SIZE_MAX: u32 = 0xfac89643; // EnemySizeMax

fn murm3(data: &[u8]) -> u32 {
    let mut h: u32 = 0xffffffff;
    let (c1, c2): (u32, u32) = (0xcc9e2d51, 0x1b873593);
    let n = data.len();
    let mut i = 0;
    while i + 4 <= n {
        let k = u32::from_le_bytes([data[i], data[i+1], data[i+2], data[i+3]]).wrapping_mul(c1).rotate_left(15).wrapping_mul(c2);
        h ^= k; h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe6546b64); i += 4;
    }
    let rem = n - i;
    let k = match rem { 3 => (data[i+2] as u32)<<16 | (data[i+1] as u32)<<8 | data[i] as u32, 2 => (data[i+1] as u32)<<8 | data[i] as u32, 1 => data[i] as u32, _ => 0 };
    if rem > 0 { h ^= k.wrapping_mul(c1).rotate_left(15).wrapping_mul(c2); }
    h ^= n as u32; h ^= h >> 16; h = h.wrapping_mul(0x85ebca6b); h ^= h >> 13; h = h.wrapping_mul(0xc2b2ae35); h ^= h >> 16; h
}
fn set_link_and_tail(buf: &mut [u8], link: u32) {
    if buf.len() >= 0x10 { buf[0x0C..0x10].copy_from_slice(&link.to_le_bytes()); }
    if buf.len() >= 4 { let l = buf.len(); let fh = murm3(&buf[..l-4]); buf[l-4..].copy_from_slice(&fh.to_le_bytes()); }
}

fn find_class<'a>(fields: &'a [(u32, Class)], hash: u32) -> Option<&'a Class> {
    fields.iter().find(|(h, _)| *h == hash).map(|(_, c)| c)
}
fn find_class_mut<'a>(fields: &'a mut [(u32, Class)], hash: u32) -> Option<&'a mut Class> {
    fields.iter_mut().find(|(h, _)| *h == hash).map(|(_, c)| c)
}
fn field<'a>(c: &'a Class, h: u32) -> Option<&'a FieldValue> {
    c.fields.iter().find(|f| f.hash == h).map(|f| &f.value)
}

fn cls_dump(c: &Class, ind: usize, depth: usize) {
    let pad = "  ".repeat(ind);
    println!("{}Class 0x{:08x} ({} fields)", pad, c.hash, c.fields.len());
    if depth == 0 { return; }
    for f in &c.fields {
        match &f.value {
            FieldValue::Class(sub) => { println!("{}- 0x{:08x}: Class", pad, f.hash); cls_dump(sub, ind+1, depth-1); }
            FieldValue::Array(a) => {
                println!("{}- 0x{:08x}: Array(len={})", pad, f.hash, a.values.len());
                if depth > 1 { for (i, v) in a.values.iter().take(24).enumerate() { println!("{}    [{}] {:?}", pad, i, v); } }
            }
            other => println!("{}- 0x{:08x}: {:?}", pad, f.hash, other),
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() >= 2 && a[1] == "clsdump" {
        // clsdump <file> <steamid> <top-hash> [depth]
        let file = &a[2]; let id: u64 = a[3].parse().unwrap();
        let top: u32 = u32::from_str_radix(a[4].trim_start_matches("0x"), 16).unwrap();
        let depth: usize = if a.len() >= 6 { a[5].parse().unwrap() } else { 2 };
        let mut o = SaveOptions::new(Game::MHRISE).id(id).curve_index(107);
        match SaveFile::load(file, &mut o) {
            Ok(sf) => {
                if let Some((_, cls)) = sf.fields.iter().find(|(h, _)| *h == top) {
                    cls_dump(cls, 0, depth);
                } else { eprintln!("top 0x{top:08x} not found"); }
            }
            Err(e) => eprintln!("load failed: {e}"),
        }
        return;
    }
    if a.len() >= 2 && a[1] == "list" {
        let file = &a[2]; let id: u64 = if a.len() >= 4 { a[3].parse().unwrap() } else { 1 };
        let mut o = SaveOptions::new(Game::MHRISE).id(id).curve_index(107);
        match SaveFile::load(file, &mut o) {
            Ok(sf) => { println!("top-level {}:", file); for (h, cls) in &sf.fields { println!("  0x{h:08x}  cls=0x{:08x}  {}", cls.hash, if *h==HUNTER_RECORD { "<-- HunterRecord" } else { "" }); } }
            Err(e) => eprintln!("load failed: {e}"),
        }
        return;
    }
    if a.len() < 5 { eprintln!("usage: size_sync <pc_slot2.bin> <switch_slot.bin> <out.bin> <steamid> [pc_sys.bin]"); std::process::exit(1); }
    let pc_path = &a[1]; let sw_path = &a[2]; let out_path = &a[3];
    let steamid: u64 = a[4].parse().unwrap();
    let sys_link = if a.len() >= 6 { let s = std::fs::read(&a[5]).unwrap_or_default(); if s.len() >= 0x10 { u32::from_le_bytes(s[0x0C..0x10].try_into().unwrap()) } else { 0 } } else { 0 };

    let mut pc_opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
    let mut pc = SaveFile::load(pc_path, &mut pc_opts).unwrap_or_else(|e| { eprintln!("load pc failed: {e}"); std::process::exit(1); });
    let mut sw_opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
    let sw = SaveFile::load(sw_path, &mut sw_opts).unwrap_or_else(|e| { eprintln!("load switch failed: {e}"); std::process::exit(1); });

    let pc_hr = find_class_mut(&mut pc.fields, HUNTER_RECORD).unwrap_or_else(|| { eprintln!("pc HunterRecord not found"); std::process::exit(1); });
    let sw_hr = find_class(&sw.fields, HUNTER_RECORD).unwrap_or_else(|| { eprintln!("switch HunterRecord not found"); std::process::exit(1); });

    let mut done = 0usize;
    for (fh, label) in [(ENEMY_SIZE_MIN, "EnemySizeMin"), (ENEMY_SIZE_MAX, "EnemySizeMax")] {
        let src = match field(sw_hr, fh) { Some(v) => v.clone(), None => { eprintln!("switch {label} not found"); continue; } };
        if let Some(f) = pc_hr.fields.iter_mut().find(|f| f.hash == fh) {
            f.value = src; done += 1; println!("patched {label}");
        } else { eprintln!("pc {label} not found"); }
    }

    pc.flags = SaveFlags::CITRUS;
    let mut bytes = pc.write_save(&pc_opts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); std::process::exit(1); });
    // keep the slot's own link if no sys given; else use sys link
    if sys_link != 0 { set_link_and_tail(&mut bytes, sys_link); }
    else if bytes.len() >= 4 { let l = bytes.len(); let fh = murm3(&bytes[..l-4]); bytes[l-4..].copy_from_slice(&fh.to_le_bytes()); }
    std::fs::write(out_path, &bytes).unwrap();
    // verify reload
    let mut vopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
    match SaveFile::load(out_path, &mut vopts) {
        Ok(s) => {
            let hr = find_class(&s.fields, HUNTER_RECORD).unwrap();
            let hc = murm3(b"HuntingCount");
            let hcval = field(hr, hc).and_then(|v| if let FieldValue::U32(x) = v { Some(*x) } else { None });
            println!("verify reload OK. HuntingCount={hcval:?}");
            for (fh, label) in [(ENEMY_SIZE_MIN, "EnemySizeMin"), (ENEMY_SIZE_MAX, "EnemySizeMax")] {
                if let Some(FieldValue::Array(a)) = field(hr, fh) {
                    let hexs: Vec<String> = a.values.iter().take(5).map(|v| match v { FieldValue::U32(x) => format!("#{x:x}"), FieldValue::U8(x) => format!("#{x:x}"), other => format!("{:?}", other) }).collect();
                    println!("{label}: len={} head={:?}", a.values.len(), hexs);
                }
            }
            println!("done {done}/2");
        }
        Err(e) => eprintln!("verify reload FAILED: {e}"),
    }
}
