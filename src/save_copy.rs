use std::process::exit;

use ree_lib::game_context::{AssetPaths, GameCtx};
use ree_lib::save::crypto::citrus::Citrus;
use ree_lib::save::game::Game;
use ree_lib::save::types::{Class, EnumValue, FieldType, FieldValue};
use ree_lib::save::{SaveFile, SaveFlags, SaveOptions};
use ree_lib::sdk::type_map::{FieldInfo, TypeInfo};
use util::murmur3;
use std::io::Write;


#[allow(dead_code)]
fn parse_scalar_fieldvalue(v: &FieldValue, kind: &str, rest: &str) -> Option<FieldValue> {
    // build a FieldValue of the same type as v from a ref VAL string
    match v {
        FieldValue::U8(_) => rest.parse::<u8>().ok().map(FieldValue::U8),
        FieldValue::U16(_) => rest.parse::<u16>().ok().map(FieldValue::U16),
        FieldValue::U32(_) => rest.parse::<u32>().ok().map(FieldValue::U32),
        FieldValue::U64(_) => rest.parse::<u64>().ok().map(FieldValue::U64),
        FieldValue::S8(_) => rest.parse::<i8>().ok().map(FieldValue::S8),
        FieldValue::S16(_) => rest.parse::<i16>().ok().map(FieldValue::S16),
        FieldValue::S32(_) => rest.parse::<i32>().ok().map(FieldValue::S32),
        FieldValue::S64(_) => rest.parse::<i64>().ok().map(FieldValue::S64),
        FieldValue::F32(_) => rest.parse::<f32>().ok().map(FieldValue::F32),
        FieldValue::F64(_) => rest.parse::<f64>().ok().map(FieldValue::F64),
        FieldValue::Boolean(_) => rest.parse::<bool>().ok().map(FieldValue::Boolean),
        FieldValue::Enum(e) => {
            let n = rest.trim().parse::<i32>().ok()?;
            let ne = match e {
                EnumValue::E1(_) => EnumValue::E1(n as i8),
                EnumValue::E2(_) => EnumValue::E2(n as i16),
                EnumValue::E4(_) => EnumValue::E4(n),
                EnumValue::E8(_) => EnumValue::E8(n as i64),
            };
            Some(FieldValue::Enum(ne))
        }
        _ => None,
    }
}


fn apply_ref_line(sf: &mut SaveFile, ctx: &GameCtx, topidx: usize, path: &str, kind: &str, rest: &str) -> bool {
    let segs: Vec<&str> = path.split('.').collect();
    let top = &mut sf.fields[topidx].1;
    let mut parts: Vec<(String, Option<usize>)> = Vec::new();
    for s in &segs {
        let (nm, idx) = if let Some(b) = s.find('[') {
            let nm = s[..b].to_string();
            let idx = s[b..].trim_start_matches('[').trim_end_matches(']').parse::<usize>().ok();
            (nm, idx)
        } else {
            ((*s).to_string(), None)
        };
        parts.push((nm, idx));
    }
    walk_class(top, &parts, 0, kind, rest)
}

fn walk_class(c: &mut Class, parts: &[(String, Option<usize>)], idx: usize, kind: &str, rest: &str) -> bool {
    if idx >= parts.len() { return false; }
    let (name, opt_index) = &parts[idx];
    let Some(fi) = c.find(name) else { return false };
    let is_last = idx == parts.len() - 1;
    if let Some(ai) = opt_index {
        // the field must be an array; navigate to element
        let arr = match &mut c.fields[fi].value {
            FieldValue::Array(a) => a,
            _ => return false,
        };
        if *ai >= arr.values.len() { return false; }
        if is_last { return false; }
        let (p1, p2) = parts.split_at(idx + 1);
        let _ = p1;
        return walk_val(&mut arr.values[*ai], p2, 0, kind, rest);
    }
    if is_last {
        return set_value(&mut c.fields[fi].value, kind, rest);
    }
    let (p1, p2) = parts.split_at(idx + 1);
    let _ = p1;
    walk_val(&mut c.fields[fi].value, p2, 0, kind, rest)
}

fn walk_val(v: &mut FieldValue, parts: &[(String, Option<usize>)], idx: usize, kind: &str, rest: &str) -> bool {
    if idx >= parts.len() { return false; }
    match v {
        FieldValue::Class(c) => walk_class(c, parts, idx, kind, rest),
        FieldValue::Array(a) => {
            // next part should be an index onto this array (a standalone [N] segment unlikely)
            let (_, opt) = &parts[idx];
            if let Some(ai) = opt {
                if *ai >= a.values.len() { return false; }
                return walk_val(&mut a.values[*ai], parts, idx + 1, kind, rest);
            }
            false
        }
        _ => false,
    }
}

fn set_value(v: &mut FieldValue, kind: &str, rest: &str) -> bool {
    if kind == "VAL" {
        match parse_scalar_fieldvalue(v, kind, rest) {
            Some(nv) => { *v = nv; true }
            None => false,
        }
    } else if kind == "ARRAY" {
        // rest = "n\tv1,v2,..."
        let (n_s, vals_s) = match rest.split_once('\t') { Some(x) => x, None => return false };
        let n = n_s.trim().parse::<usize>().unwrap_or(0);
        let vals: Vec<i64> = vals_s.split(',').filter_map(|x| x.trim().parse::<i64>().ok()).collect();
        // build Array matching the existing type
        if let FieldValue::Array(a) = v {
            let mut newvals: Vec<FieldValue> = Vec::new();
            for (i, x) in vals.iter().enumerate() {
                let base = if let Some(b) = a.values.get(i) { b.clone() }
                    else if let Some(b) = a.values.first() { b.clone() }
                    else { match a.member_type {
                        FieldType::U8 => FieldValue::U8(0),
                        FieldType::U16 => FieldValue::U16(0),
                        FieldType::U32 => FieldValue::U32(0),
                        FieldType::U64 => FieldValue::U64(0),
                        FieldType::S8 => FieldValue::S8(0),
                        FieldType::S16 => FieldValue::S16(0),
                        FieldType::S32 => FieldValue::S32(0),
                        FieldType::S64 => FieldValue::S64(0),
                        FieldType::Boolean => FieldValue::Boolean(false),
                        FieldType::Enum => FieldValue::Enum(EnumValue::E4(0)),
                        _ => return false,
                    } };
                newvals.push(match &base {
                    FieldValue::U32(_) => FieldValue::U32((*x as u32)),
                    FieldValue::S32(_) => FieldValue::S32((*x as i32)),
                    FieldValue::U64(_) => FieldValue::U64((*x as u64)),
                    FieldValue::S64(_) => FieldValue::S64(*x),
                    FieldValue::U16(_) => FieldValue::U16((*x as u16)),
                    FieldValue::S16(_) => FieldValue::S16((*x as i16)),
                    FieldValue::U8(_) => FieldValue::U8((*x as u8)),
                    FieldValue::S8(_) => FieldValue::S8((*x as i8)),
                    FieldValue::Enum(e) => {
                        let ne = match e {
                            EnumValue::E1(_) => EnumValue::E1(*x as i8),
                            EnumValue::E2(_) => EnumValue::E2(*x as i16),
                            EnumValue::E4(_) => EnumValue::E4(*x as i32),
                            EnumValue::E8(_) => EnumValue::E8(*x),
                        };
                        FieldValue::Enum(ne)
                    }
                    FieldValue::Boolean(b) => FieldValue::Boolean(*x != 0),
                    _ => FieldValue::S32((*x as i32)),
                });
            }
            while newvals.len() < n {
                let base = if a.values.is_empty() { FieldValue::U32(0) } else { a.values[0].clone() };
                newvals.push(base);
            }
            a.values = newvals;
            true
        } else {
            false
        }
    } else { false }
}

fn typeinfo_for<'a>(ctx: &'a GameCtx, _class: &Class) -> Option<&'a TypeInfo> {
    ctx.type_map.get_by_hash(_class.hash)
}

fn field_name(ti: &TypeInfo, hash: u32) -> String {
    ti.fields
        .get(&hash)
        .map(|f: &FieldInfo| f.name.clone())
        .unwrap_or_else(|| format!("<0x{:08x}>", hash))
}

fn dump_class(ctx: &GameCtx, c: &Class, indent: usize, depth: usize) {
    let pad = "  ".repeat(indent);
    let name = typeinfo_for(ctx, c)
        .map(|t| t.name.clone())
        .unwrap_or_else(|| format!("<cls 0x{:08x}>", c.hash));
    println!("{}Class {} (hash=0x{:08x}, {} fields)", pad, name, c.hash, c.fields.len());
    if depth == 0 {
        return;
    }
    let ti = typeinfo_for(ctx, c);
    for f in &c.fields {
        let fname = ti
            .map(|t| field_name(t, f.hash))
            .unwrap_or_else(|| format!("<0x{:08x}>", f.hash));
        match &f.value {
            FieldValue::Class(sub) => {
                println!("{}- {}:", pad, fname);
                dump_class(ctx, sub, indent + 1, depth - 1);
            }
            FieldValue::Array(a) => {
                println!("{}- {}: Array(len={}, type={:?})", pad, fname, a.values.len(), a.array_type);
                if depth > 1 {
                    for (i, v) in a.values.iter().enumerate().take(8) {
                        if let FieldValue::Class(sub) = v {
                            println!("{}  [{}]:", pad, i);
                            dump_class(ctx, sub, indent + 2, depth - 2);
                        } else {
                            println!("{}  [{}]: {:?}", pad, i, v);
                        }
                    }
                }
            }
            other => {
                println!("{}- {}: {:?}", pad, fname, other);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    let mut cmd = String::new();
    let mut save = String::new();
    let mut steamid: u64 = 0;
    let mut depth = 2usize;
    let mut top: Option<usize> = None;
    let mut src = String::new();
    let mut out = String::new();
    let mut sys = String::new();
    let mut ref_file = String::new();
    let mut curve_index: usize = 107;
    let mut switchsys = String::new();
    let mut src_tail = String::new();
    let mut sw_slot = 0usize;
    let mut n_slice = 0usize;
    let mut hash_top: u32 = 0;
    let mut named_mode = false;
    let mut pc_slot = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "dump" => cmd = "dump".to_string(),
            "copy" => cmd = "copy".to_string(),
            "convert" => cmd = "convert".to_string(),
            "convert-raw" => cmd = "convert-raw".to_string(),
            "clear-ids" => cmd = "clear-ids".to_string(),
            "resave" => cmd = "resave".to_string(),
            "slice" => cmd = "slice".to_string(),
            "extract-items" => cmd = "extract-items".to_string(),
            "extract-equip" => cmd = "extract-equip".to_string(),
            "extract-talismans" => cmd = "extract-talismans".to_string(),
            "extract-deco" => cmd = "extract-deco".to_string(),
            "extract-aug" => cmd = "extract-aug".to_string(),
            "extract-quest" => cmd = "extract-quest".to_string(),
            "set-village" => cmd = "set-village".to_string(),
            "clear-slot" => cmd = "clear-slot".to_string(),
            "extract-npc" => cmd = "extract-npc".to_string(),
            "extract-flags" => cmd = "extract-flags".to_string(),
            "extract-class" => cmd = "extract-class".to_string(),
            "offline-apply" => cmd = "offline-apply".to_string(),
            "patch-sys" => cmd = "patch-sys".to_string(),
            "--save" => { i += 1; save = args[i].clone(); }
            "--steamid" => { i += 1; steamid = args[i].parse().unwrap(); }
            "--depth" => { i += 1; depth = args[i].parse().unwrap(); }
            "--top" => { i += 1; top = Some(args[i].parse().unwrap()); }
            "--src" => { i += 1; src = args[i].clone(); }
            "--out" => { i += 1; out = args[i].clone(); }
            "--curve" => { i += 1; curve_index = args[i].parse().unwrap(); }
            "--ref" => { i += 1; ref_file = args[i].clone(); }
            "--sys" => { i += 1; sys = args[i].clone(); }
            "--switchsys" => { i += 1; switchsys = args[i].clone(); }
            "--sw-slot" => { i += 1; sw_slot = args[i].parse().unwrap(); }
            "--pc-slot" => { i += 1; pc_slot = args[i].parse().unwrap(); }
            "--idx" => { i += 1; pc_slot = args[i].parse().unwrap(); }
            "--village" => { i += 1; sw_slot = args[i].parse().unwrap(); }
            "--hash" => { i += 1; hash_top = u32::from_str_radix(&args[i].trim_start_matches("0x"), 16).unwrap(); }
            "--named" => { named_mode = true; }
            "--tail" => { i += 1; src_tail = args[i].clone(); }
            "--n" => { i += 1; n_slice = args[i].parse().unwrap(); }
            _ => {}
        }
        i += 1;
    }

    const LOADINFO_HASH: u32 = 0x0b19f75b;

    if cmd == "patch-sys" {
        if sys.is_empty() || switchsys.is_empty() || out.is_empty() || steamid == 0 {
            eprintln!("usage: save_copy patch-sys --sys <pc data00-1.bin> --switchsys <switch data00-1.bin> --sw-slot N --pc-slot N --steamid <id64> --out <outfile>");
            exit(1);
        }
        let _ctx = GameCtx::new(&AssetPaths::from_game(Game::MHRISE));
        let mut popts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let mut pc_sys = SaveFile::load(&sys, &mut popts).unwrap_or_else(|e| { eprintln!("load pc sys failed: {e}"); exit(1); });
        let mut sopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sw_sys = SaveFile::load(&switchsys, &mut sopts).unwrap_or_else(|e| { eprintln!("load switch sys failed: {e}"); exit(1); });

        // source: switch LoadInfoData._HunterArray[sw_slot]
        let sw_li = sw_sys.fields.iter().find(|(h, _)| *h == LOADINFO_HASH).map(|(_, c)| c).expect("switch LoadInfoData not found");
        let sw_src: Class = match sw_li.get_value("_HunterArray") {
            Some(FieldValue::Array(a)) => a.get_as_class(sw_slot).expect("switch slot out of range").clone(),
            _ => panic!("switch _HunterArray not an array"),
        };

        // target: pc LoadInfoData._HunterArray[pc_slot]
        let pc_li = pc_sys.fields.iter_mut().find(|(h, _)| *h == LOADINFO_HASH).map(|(_, c)| c).expect("pc LoadInfoData not found");
        let pc_arr = match pc_li.get_value_mut("_HunterArray") {
            Some(FieldValue::Array(a)) => a,
            _ => panic!("pc _HunterArray not an array"),
        };
        // preserve target _UseIndex
        let keep_use: Option<i32> = pc_arr.get_as_class(pc_slot)
            .and_then(|c| match c.get_value("_UseIndex") { Some(FieldValue::S32(v)) => Some(*v), _ => None });
        if let Some(FieldValue::Class(bx)) = pc_arr.values.get_mut(pc_slot) {
            **bx = sw_src.clone();
        } else {
            panic!("pc slot not a class");
        }
        if let Some(use_idx) = keep_use {
            if let Some(FieldValue::S32(v)) = pc_arr.get_as_class_mut(pc_slot).and_then(|c| c.get_value_mut("_UseIndex")) {
                *v = use_idx;
            }
        }

        pc_sys.save(&out, &popts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); exit(1); });
        let data = std::fs::read(&out).unwrap();
        println!("Wrote {} ({} bytes), flags={:04x}", out, data.len(), data[8] as u32);
        let mut vopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        match SaveFile::load(&out, &mut vopts) {
            Ok(_) => println!("Verify reload OK"),
            Err(e) => eprintln!("Verify reload FAILED: {e}"),
        }
        return;
    }

    if cmd == "convert-raw" {
        if src.is_empty() || out.is_empty() || steamid == 0 {
            eprintln!("usage: save_copy convert-raw --src <switch save> --steamid <id64> --out <outfile>");
            exit(1);
        }
        let bytes = std::fs::read(&src).unwrap_or_else(|e| { eprintln!("read failed: {e}"); exit(1); });
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let (data, data_offset, _bf, _fl) = SaveFile::process_bytes_to_stream(bytes, &mut opts)
            .unwrap_or_else(|e| { eprintln!("process failed: {e}"); exit(1); });
        let raw_classes = &data[data_offset as usize..];
        println!("raw serialized payload: {} bytes", raw_classes.len());

        let mut file = Vec::new();
        file.extend_from_slice(b"DSSS");
        file.extend_from_slice(&2u32.to_le_bytes());
        file.extend_from_slice(&SaveFlags::CITRUS.bits().to_le_bytes());
        file.extend_from_slice(&0x208u32.to_le_bytes());

        let decrypted_size = raw_classes.len() as u64;
        let id = opts.id.unwrap_or(steamid);
        let citrus = Citrus::new(id, opts.curve_index);
        let mut payload = citrus.encrypt(raw_classes).expect("citrus encrypt failed");
        payload.extend_from_slice(&decrypted_size.to_le_bytes());
        file.extend_from_slice(&payload);
        while file.len() % 4 != 0 { file.push(0); }

        if !src_tail.is_empty() {
            let ref_data = std::fs::read(&src_tail).unwrap_or_else(|e| { eprintln!("read tail ref failed: {e}"); exit(1); });
            let n = ref_data.len();
            let mut rd_off = 0usize;
            for i in (8..=n.saturating_sub(8)).rev() {
                let val = u64::from_le_bytes(ref_data[i..i+8].try_into().unwrap());
                if (0x2000..n as u64).contains(&val) && val as usize + 0x1000 + 12 <= n {
                    rd_off = i; break;
                }
            }
            if rd_off >= 0x1000 {
                let ref_tail = &ref_data[rd_off-0x1000..rd_off];
                let num_blocks = raw_classes.len().div_ceil(0x40000);
                let tail_offset = 16 + num_blocks * 0x40240;
                if tail_offset + 0x1000 <= file.len() {
                    file[tail_offset..tail_offset+0x1000].copy_from_slice(ref_tail);
                    println!("copied {} byte tail from {}", 0x1000, src_tail);
                } else {
                    println!("WARN tail_offset out of range");
                }
            } else {
                println!("WARN could not find tail in reference");
            }
        }

        let file_hash = murmur3(&file, 0xffffffff);
        file.extend_from_slice(&file_hash.to_le_bytes());
        std::fs::write(&out, &file).unwrap();
        println!("Wrote {} ({} bytes), flags={:04x}", out, file.len(), file[8] as u32);

        let mut vopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        match SaveFile::load(&out, &mut vopts) {
            Ok(sf) => println!("Verify reload OK: {} top-level fields, curve={:?}", sf.fields.len(), vopts.curve_index),
            Err(e) => eprintln!("Verify reload FAILED: {e}"),
        }
        return;
    }

    if cmd == "clear-ids" {
        if src.is_empty() || out.is_empty() || steamid == 0 {
            eprintln!("usage: save_copy clear-ids --src <switch save> --steamid <id64> --out <outfile>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let mut sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });

        fn zero_struct_field(c: &mut Class, name: &str) {
            if let Some(FieldValue::Struct(s)) = c.get_value_mut(name) {
                s.data.fill(0);
            }
        }
        fn zero_u8_array_by_hash(c: &mut Class, hash: u32) {
            for f in c.fields.iter_mut() {
                if f.hash == hash {
                    if let FieldValue::Array(a) = &mut f.value {
                        for v in a.values.iter_mut() {
                            if let FieldValue::U8(x) = v { *x = 0; }
                        }
                    }
                }
            }
        }
        fn zero_string_field(c: &mut Class, name: &str) {
            if let Some(FieldValue::String(s)) = c.get_value_mut(name) {
                s.0.fill(0);
            }
        }

        // snow.HunterRecordManager.HunterReocrdSaveData (top-level field hash 0x355c8c4f)
        if let Some(c) = sf.fields.iter_mut().find(|(h, _)| *h == 0x355c8c4f).map(|(_, c)| c) {
            zero_struct_field(c, "HunterUniqueID");
            zero_string_field(c, "NsaID");
            if let Some(FieldValue::Class(bin)) = c.get_value_mut("NetworkUniqueId_Binary") {
                zero_u8_array_by_hash(bin, 0xef5095c4);
            }
            println!("cleared IDs in HunterReocrdSaveData");
        }

        sf.flags = SaveFlags::CITRUS;
        sf.save(&out, &opts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); exit(1); });
        let data = std::fs::read(&out).unwrap();
        println!("Wrote {} ({} bytes), flags={:04x}", out, data.len(), data[8] as u32);
        let mut vopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        match SaveFile::load(&out, &mut vopts) {
            Ok(s) => println!("Verify reload OK: {} top-level fields", s.fields.len()),
            Err(e) => eprintln!("Verify reload FAILED: {e}"),
        }
        return;
    }

    if cmd == "resave" || cmd == "slice" {
        if src.is_empty() || out.is_empty() || steamid == 0 {
            eprintln!("usage: save_copy resave --src <pc save> --steamid <id64> --out <out>");
            eprintln!("       save_copy slice  --src <pc save> --sys <switch save> --n <N> --steamid <id64> --out <out>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let pc_path = src.clone();
        let mut pc = SaveFile::load(&pc_path, &mut opts).unwrap_or_else(|e| { eprintln!("load pc save failed: {e}"); exit(1); });

        if cmd == "slice" {
            if sys.is_empty() {
                eprintln!("slice requires --sys <switch save>"); exit(1);
            }
            let mut sopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
            let sw = SaveFile::load(&sys, &mut sopts).unwrap_or_else(|e| { eprintln!("load switch save failed: {e}"); exit(1); });
            let mut copied = 0;
            if hash_top != 0 {
                if let Some((h, cls)) = sw.fields.iter().find(|(h, _)| *h == hash_top) {
                    if let Some((_, pc_cls)) = pc.fields.iter_mut().find(|(ph, _)| ph == h) {
                        *pc_cls = cls.clone();
                        copied = 1;
                    }
                }
            } else {
                let n = n_slice.min(sw.fields.len());
                for (h, cls) in sw.fields.iter().take(n) {
                    if let Some((_, pc_cls)) = pc.fields.iter_mut().find(|(ph, _)| ph == h) {
                        *pc_cls = cls.clone();
                        copied += 1;
                    }
                }
            }
            println!("sliced: copied {copied} top-level classes from Switch into PC save (hash={hash_top:#x})");
        }

        pc.flags = SaveFlags::CITRUS;
        let src_link = std::fs::read(&pc_path).ok()
            .filter(|d| d.len() >= 0x10)
            .map(|d| u32::from_le_bytes(d[0x0C..0x10].try_into().unwrap()))
            .unwrap_or(0u32);
        pc.save(&out, &opts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); exit(1); });
        let mut out_data = std::fs::read(&out).unwrap();
        {
            // preserve the ORIGINAL save link value (0x0C) from the source slot
            let src_raw = std::fs::read(&pc_path).unwrap_or_default();
            let link = src_link;
            let hdr = &mut out_data[0x0C..0x10];
            hdr.copy_from_slice(&link.to_le_bytes());
            println!("0x0C link = {link}");
            // also patch the system file (data00-1.bin) to the same link if given via --sys
            if !sys.is_empty() {
                match std::fs::read(&sys) {
                    Ok(mut sysd) if sysd.len() >= 0x10 => {
                        let (a, b) = (sysd[0x0C..0x10].to_vec(), link.to_le_bytes());
                        let _ = (a, b);
                        sysd[0x0C..0x10].copy_from_slice(&link.to_le_bytes());
                        let syslen = sysd.len();
                        let fh = murmur3(&sysd[..syslen-4], 0xffffffff);
                        sysd[syslen-4..].copy_from_slice(&fh.to_le_bytes());
                        std::fs::write(&sys, &sysd).unwrap();
                        println!("patched system file 0x0C link -> {link} (and re-hashed tail)");
                    }
                    other => eprintln!("sys read skipped: {}", other.err().map(|e| e.to_string()).unwrap_or_else(|| "?".into())),
                }
            }
        }
        let len = out_data.len();
        let fh = murmur3(&out_data[..len-4], 0xffffffff);
        out_data[len-4..].copy_from_slice(&fh.to_le_bytes());
        std::fs::write(&out, &out_data).unwrap();
        let data = std::fs::read(&out).unwrap();
        println!("Wrote {} ({} bytes), flags={:04x}", out, data.len(), data[8] as u32);
        let mut vopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        match SaveFile::load(&out, &mut vopts) {
            Ok(s) => println!("Verify reload OK: {} top-level fields", s.fields.len()),
            Err(e) => eprintln!("Verify reload FAILED: {e}"),
        }
        return;
    }


    // ---- offline-apply: set fields from a named reference into a save file, then relink+rehash ----
    if cmd == "offline-apply" {
        if src.is_empty() || out.is_empty() || hash_top == 0 || ref_file.is_empty() {
            eprintln!("usage: save_copy offline-apply --src <pc save> --hash <top hash> --ref <reference.txt> --steamid <id64> --out <out> [--sys <data00-1.bin>]");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let mut sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        let ctx = GameCtx::new(&AssetPaths::from_game(Game::MHRISE));
        let mut topidx = None;
        for (i, (h, _)) in sf.fields.iter().enumerate() {
            if *h == hash_top { topidx = Some(i); break; }
        }
        if topidx.is_none() { eprintln!("top hash not found"); exit(1); }
        let content = std::fs::read_to_string(&ref_file).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        // build list of (path, Option<(kind, rest)>)
        let mut entries: Vec<(String, Option<String>, Option<String>)> = Vec::new();
        // pairs: bare path line then path\tVAL/ARRAY line
        for l in &lines {
            let t = l.trim_end();
            if t.is_empty() { continue; }
            let parts: Vec<&str> = t.splitn(3, '\t').collect();
            if parts.len() == 1 {
                entries.push((parts[0].to_string(), None, None));
            } else if parts.len() >= 3 {
                entries.push((parts[0].to_string(), Some(parts[1].to_string()), Some(parts[2].to_string())));
            }
        }
        let mut setc = 0usize;
        let mut failc = 0usize;
        for (path, kind, rest) in &entries {
            let (value_kind, value_rest) = match (kind.as_ref(), rest.as_ref()) {
                (Some(k), Some(r)) => (k.clone(), r.clone()),
                _ => continue,
            };
            let okay = apply_ref_line(&mut sf, &ctx, topidx.unwrap(), path, &value_kind, &value_rest);
            if okay { setc += 1; } else { failc += 1; }
        }
        println!("offline-apply: set {setc}, failed {failc}");
        // save + relink + rehash (same as resave)
        sf.flags = SaveFlags::CITRUS;
        let src_link = std::fs::read(&src).ok()
            .filter(|d| d.len() >= 0x10)
            .map(|d| u32::from_le_bytes(d[0x0C..0x10].try_into().unwrap()))
            .unwrap_or(0u32);
        sf.save(&out, &opts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); exit(1); });
        let mut out_data = std::fs::read(&out).unwrap();
        out_data[0x0C..0x10].copy_from_slice(&src_link.to_le_bytes());
        let len = out_data.len();
        let fh = murmur3(&out_data[..len - 4], 0xffffffff);
        out_data[len - 4..].copy_from_slice(&fh.to_le_bytes());
        std::fs::write(&out, &out_data).unwrap();
        println!("wrote {} (link {src_link}, {} bytes)", out, len);
        if !sys.is_empty() {
            if let Ok(mut sysd) = std::fs::read(&sys) {
                if sysd.len() >= 0x10 {
                    sysd[0x0C..0x10].copy_from_slice(&src_link.to_le_bytes());
                    let sl = sysd.len();
                    let fh = murmur3(&sysd[..sl - 4], 0xffffffff);
                    sysd[sl - 4..].copy_from_slice(&fh.to_le_bytes());
                    std::fs::write(&sys, &sysd).unwrap();
                    println!("patched sys {} link -> {src_link}", sys);
                }
            }
        }
        return;
    }


    // ---- migrate: slice ALL top-level classes from a Switch save into a PC save, then relink+rehash ----
    if cmd == "migrate" {
        if src.is_empty() || out.is_empty() || sys.is_empty() || steamid == 0 {
            eprintln!("usage: save_copy migrate --src <pc slot.bin> --switchsys <switch slot.bin> --sys <pc data00-1.bin> --steamid <id64> --curve <curve> --out <out slot.bin>");
            eprintln!("       (writes the patched slot to --out and patches the PC data00-1.bin link in place)");
            exit(1);
        }
        let all: Vec<u32> = vec![
            0xd6f4726d, 0x9ccc3b1e, 0xb0ca70c9, 0xf2fb669a, 0x1e32797d, 0x355c8c4f,
            0x1f613294, 0x164d2e71, 0x51dcd6fb, 0xb9c15dcf, 0xadc075b6, 0x20d9167c,
            0x93819625, 0x8512ab74, 0x553d33b8, 0x68344f29, 0x8c6fb4c6, 0x1322883a,
            0xa81238c6, 0xaaca38e3, 0x22a8b022, 0x3da5b9de, 0xa21e01ad, 0x0386f39d,
            0xe825861b, 0x356f270b, 0xef78287f, 0xf5e018a4, 0xdc00f45c, 0xddb8e034,
            0x4423bc21, 0xcf6c5091, 0x81fcc8f4, 0x0aa?0, 0x00
        ];
        let all: Vec<u32> = vec![
            0xd6f4726d, 0x9ccc3b1e, 0xb0ca70c9, 0xf2fb669a, 0x1e32797d, 0x355c8c4f,
            0x1f613294, 0x164d2e71, 0x51dcd6fb, 0xb9c15dcf, 0xadc075b6, 0x20d9167c,
            0x93819625, 0x8512ab74, 0x553d33b8, 0x68344f29, 0x8c6fb4c6, 0x1322883a,
            0xa81238c6, 0xaaca38e3, 0x22a8b022, 0x3da5b9de, 0xa21e01ad, 0x0386f39d,
            0xe825861b, 0x356f270b, 0xef78287f, 0xf5e018a4, 0xdc00f45c, 0xddb8e034,
            0x4423bc21, 0xcf6c5091, 0x81fcc8f4, 0xab109098, 0xd5f91c48,
        ];
        let curve = curve_index as usize;
        // account-bound classes intentionally EXCLUDED: network.SaveData (0xe58fb6c9), Dlc (0x1f613294 listed? - skip Dlc too? keep it simple: exclude Dlc & network)
        let all: Vec<u32> = all.into_iter().filter(|h| *h != 0x1f613294).collect();
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(curve);
        let mut pc = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load pc save failed: {e}"); exit(1); });
        let mut sopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(curve);
        let sw = SaveFile::load(&switchsys, &mut sopts).unwrap_or_else(|e| { eprintln!("load switch save failed: {e}"); exit(1); });
        let mut copied = 0usize;
        let mut missing = 0usize;
        for h in &all {
            if let Some((_, cls)) = sw.fields.iter().find(|(h2, _)| h2 == h) {
                if let Some((_, pc_cls)) = pc.fields.iter_mut().find(|(ph, _)| ph == h) {
                    *pc_cls = cls.clone();
                    copied += 1;
                }
            } else {
                missing += 1;
            }
        }
        println!("migrate: sliced {copied} classes from switch into pc ({missing} not found in switch)");
        pc.flags = SaveFlags::CITRUS;
        let src_link = std::fs::read(&sys).ok()
            .filter(|d| d.len() >= 0x10)
            .map(|d| u32::from_le_bytes(d[0x0C..0x10].try_into().unwrap()))
            .unwrap_or(0u32);
        pc.save(&out, &opts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); exit(1); });
        let mut out_data = std::fs::read(&out).unwrap();
        if out_data.len() >= 0x10 {
            out_data[0x0C..0x10].copy_from_slice(&src_link.to_le_bytes());
        }
        let len = out_data.len();
        let fh = murmur3(&out_data[..len - 4], 0xffffffff);
        out_data[len - 4..].copy_from_slice(&fh.to_le_bytes());
        std::fs::write(&out, &out_data).unwrap();
        println!("migrate: wrote {} (link {src_link}, {} bytes)", out, len);
        // patch the pc data00-1 link to match
        if !sys.is_empty() {
            if let Ok(mut sysd) = std::fs::read(&sys) {
                if sysd.len() >= 0x10 {
                    sysd[0x0C..0x10].copy_from_slice(&src_link.to_le_bytes());
                    let sl = sysd.len();
                    let fh = murmur3(&sysd[..sl - 4], 0xffffffff);
                    sysd[sl - 4..].copy_from_slice(&fh.to_le_bytes());
                    std::fs::write(&sys, &sysd).unwrap();
                    println!("migrate: patched {} link -> {src_link}", sys);
                }
            }
        }
        return;
    }

    if cmd == "extract-items" {
        if src.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy extract-items --src <switch save> --steamid <id64> --out <file>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        let dm = sf.fields.iter().find(|(h,_)| *h == 0xe825861b).map(|(_,c)| c)
            .expect("DataManager.SaveDataParent not found");
        let box_arr = match dm.get_subclass("_Data1").and_then(|d| d.get_subclass("_ItemBox"))
            .and_then(|b| b.get_value("_InventoryList")) {
            Some(FieldValue::Array(a)) => a,
            _ => panic!("item box _InventoryList not an array"),
        };
        let pouch_arr = match dm.get_subclass("_Data1").and_then(|d| d.get_subclass("_ItemPouch"))
            .and_then(|p| p.get_value("_NormalInventoryList")) {
            Some(FieldValue::Array(a)) => a,
            _ => panic!("item pouch _NormalInventoryList not an array"),
        };
        let mut f = std::fs::File::create(&out).unwrap();
        use std::io::Write;
        let mut dump = |a: &ree_lib::save::types::Array, label: &str| {
            for (i, v) in a.values.iter().enumerate() {
                if let FieldValue::Class(c) = v {
                    let cnt = c.get_subclass("_ItemCount");
                    let id = cnt.and_then(|c| match c.get_value("_Id") {
                        Some(FieldValue::Enum(e)) => Some(e.as_i64()), _ => None });
                    let num = cnt.and_then(|c| match c.get_value("_Num") {
                        Some(FieldValue::S32(n)) => Some(*n as i64), _ => None });
                    let _ = writeln!(f, "{}\t{}\t{}\t{}", label, i, id.unwrap_or(-1), num.unwrap_or(-1));
                }
            }
        };
        dump(&box_arr, "box");
        dump(&pouch_arr, "pouch");
        println!("wrote item reference to {}", out);
        return;
    }

    if cmd == "extract-equip" {
        if src.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy extract-equip --src <switch save> --steamid <id64> --out <file>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        let dm = sf.fields.iter().find(|(h,_)| *h == 0xe825861b).map(|(_,c)| c)
            .expect("DataManager.SaveDataParent not found");
        let arr = match dm.get_subclass("_Data1").and_then(|d| d.get_subclass("_EquipBox"))
            .and_then(|b| b.get_value("_WeaponArmorInventoryList")) {
            Some(FieldValue::Array(a)) => a,
            _ => panic!("equip box _WeaponArmorInventoryList not an array"),
        };
        use std::io::Write;
        let mut f = std::fs::File::create(&out).unwrap();
        let mut filled = 0;
        for (i, v) in arr.values.iter().enumerate() {
            if let FieldValue::Class(c) = v {
                let idtype = c.get_value("_IdType").and_then(|x| match x { FieldValue::Enum(e) => Some(e.as_i64()), _ => None });
                let idval = c.get_value("_IdVal").and_then(|x| match x { FieldValue::U32(u) => Some(*u as i64), _ => None });
                let _ = writeln!(f, "{}\t{}\t{}", i, idtype.unwrap_or(-1), idval.unwrap_or(-1));
                if idval.unwrap_or(0) > 0 { filled += 1; }
            }
        }
        println!("equip box: {} entries, {} filled (idval>0)", arr.values.len(), filled);
        println!("wrote equip reference to {}", out);
        return;
    }

    if cmd == "extract-talismans" {
        if src.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy extract-talismans --src <switch save> --steamid <id64> --out <file>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        let dm = sf.fields.iter().find(|(h,_)| *h == 0xe825861b).map(|(_,c)| c)
            .expect("DataManager.SaveDataParent not found");
        let arr = match dm.get_subclass("_Data1").and_then(|d| d.get_subclass("_EquipBox"))
            .and_then(|b| b.get_value("_WeaponArmorInventoryList")) {
            Some(FieldValue::Array(a)) => a,
            _ => panic!("equip box not an array"),
        };
        use std::io::Write;
        let mut f = std::fs::File::create(&out).unwrap();
        let mut count = 0;
        for (i, v) in arr.values.iter().enumerate() {
            if let FieldValue::Class(c) = v {
                let idtype = c.get_value("_IdType").and_then(|x| match x { FieldValue::Enum(e) => Some(e.as_i64()), _ => None });
                let idval = c.get_value("_IdVal").and_then(|x| match x { FieldValue::U32(u) => Some(*u as i64), _ => None });
                // talisman skills
                let slv: Vec<String> = c.get_value("_TalismanSkillLvList").map(|v| match v {
                    FieldValue::Array(a) => a.values.iter().map(|x| match x { FieldValue::U32(u) => u.to_string(), _ => "-".into() }).collect(),
                    _ => vec!["-".into()],
                }).unwrap_or_default();
                let sid: Vec<String> = c.get_value("_TalismanSkillIdList").map(|v| match v {
                    FieldValue::Array(a) => a.values.iter().map(|x| match x { FieldValue::Enum(e) => e.as_i64().to_string(), _ => "-".into() }).collect(),
                    _ => vec!["-".into()],
                }).unwrap_or_default();
                let dslot: Vec<String> = c.get_value("_TalismanDecoSlotNumList").map(|v| match v {
                    FieldValue::Array(a) => a.values.iter().map(|x| match x { FieldValue::U32(u) => u.to_string(), _ => "-".into() }).collect(),
                    _ => vec!["-".into()],
                }).unwrap_or_default();
                let lvs = slv.join(",");
                let ids = sid.join(",");
                let slots = dslot.join(",");
                let _ = writeln!(f, "{}\t{}\t{}\t{}\t{}\t{}", i, idtype.unwrap_or(-1), idval.unwrap_or(-1), ids, lvs, slots);
                // count entries with any skill level > 0
                if slv.iter().any(|s| s.parse::<u32>().map(|v| v>0).unwrap_or(false)) {
                    count += 1;
                }
            }
        }
        println!("talismans (entries with skill lv>0): {}", count);
        println!("wrote talisman reference to {}", out);
        return;
    }

    if cmd == "extract-deco" {
        if src.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy extract-deco --src <switch save> --steamid <id64> --out <file>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        let dm = sf.fields.iter().find(|(h,_)| *h == 0xe825861b).map(|(_,c)| c)
            .expect("DataManager.SaveDataParent not found");
        let arr = match dm.get_subclass("_Data1").and_then(|d| d.get_subclass("_EquipBox"))
            .and_then(|b| b.get_value("_WeaponArmorInventoryList")) {
            Some(FieldValue::Array(a)) => a,
            _ => panic!("equip box not an array"),
        };
        use std::io::Write;
        let mut f = std::fs::File::create(&out).unwrap();
        let mut filled = 0;
        for (i, v) in arr.values.iter().enumerate() {
            if let FieldValue::Class(c) = v {
                let idval = c.get_value("_IdVal").and_then(|x| match x { FieldValue::U32(u) => Some(*u as i64), _ => None });
                if idval.unwrap_or(0) <= 0 { continue; }
                let deco: Vec<String> = c.get_value("_DecoIdList").map(|v| match v {
                    FieldValue::Array(a) => a.values.iter().map(|x| match x { FieldValue::Enum(e) => e.as_i64().to_string(), _ => "-".into() }).collect(),
                    _ => vec!["-".into()],
                }).unwrap_or_default();
                let hyak = c.get_value("_HyakuryuDecoId").and_then(|x| match x { FieldValue::Enum(e) => Some(e.as_i64()), _ => None });
                let _ = writeln!(f, "{}\t{}\t{}", i, deco.join(","), hyak.unwrap_or(-1));
                filled += 1;
            }
        }
        println!("filled equip entries: {}", filled);
        println!("wrote deco reference to {}", out);
        return;
    }

    if cmd == "extract-aug" {
        if src.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy extract-aug --src <switch save> --steamid <id64> --out <file>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        let dm = sf.fields.iter().find(|(h,_)| *h == 0xe825861b).map(|(_,c)| c)
            .expect("DataManager.SaveDataParent not found");
        let arr = match dm.get_subclass("_Data1").and_then(|d| d.get_subclass("_EquipBox"))
            .and_then(|b| b.get_value("_WeaponArmorInventoryList")) {
            Some(FieldValue::Array(a)) => a,
            _ => panic!("equip box not an array"),
        };
        use std::io::Write;
        let mut f = std::fs::File::create(&out).unwrap();
        let mut augmented = 0;
        for (i, v) in arr.values.iter().enumerate() {
            if let FieldValue::Class(c) = v {
                let idval = c.get_value("_IdVal").and_then(|x| match x { FieldValue::U32(u) => Some(*u as i64), _ => None });
                if idval.unwrap_or(0) <= 0 { continue; }
                let cus_en = c.get_value("_CustomEnable").and_then(|x| match x { FieldValue::Boolean(b) => Some(*b), _ => None }).unwrap_or(false);
                let cus_cnt = c.get_value("_CustomCount").and_then(|x| match x { FieldValue::S32(s) => Some(*s), _ => None }).unwrap_or(0);
                let cus_type = c.get_value("_CustomBuildupType").and_then(|x| match x { FieldValue::S8(s) => Some(*s as i64), _ => None }).unwrap_or(-1);
                let openarr: Vec<String> = c.get_value("_CustomOpenIdArray").map(|v| match v {
                    FieldValue::Array(a) => a.values.iter().map(|x| match x { FieldValue::U32(u) => u.to_string(), FieldValue::Enum(e) => e.as_i64().to_string(), _ => "-".into() }).collect(),
                    _ => vec!["-".into()],
                }).unwrap_or_default();
                let mut bu = Vec::new();
                if let Some(FieldValue::Array(a)) = c.get_value("_CustomBuildup") {
                    for (bi, bv) in a.values.iter().enumerate().take(7) {
                        if let FieldValue::Class(bc) = bv {
                            let bid = bc.get_value("_Id").and_then(|x| match x { FieldValue::Enum(e) => Some(e.as_i64()), _ => None }).unwrap_or(0);
                            let bval = bc.get_value("_ValueIndex").and_then(|x| match x { FieldValue::U32(u) => Some(*u as i64), _ => None }).unwrap_or(0);
                            let bskill = bc.get_value("_SkillId").and_then(|x| match x { FieldValue::Enum(e) => Some(e.as_i64()), _ => None }).unwrap_or(0);
                            bu.push(format!("{}:{}:{}", bid, bval, bskill));
                        }
                    }
                }
                let has_aug = cus_en || cus_cnt > 0 || bu.iter().any(|s| s != "0:0:0");
                if has_aug {
                    let _ = writeln!(f, "{}\t{}\t{}\t{}\t{}\t{}", i,
                        if cus_en {1} else {0}, cus_cnt, cus_type, openarr.join(","), bu.join(";"));
                    augmented += 1;
                }
            }
        }
        println!("augmented entries: {}", augmented);
        println!("wrote augment reference to {}", out);
        return;
    }

    if cmd == "extract-quest" {
        if src.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy extract-quest --src <save> --steamid <id64> --out <file>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        use std::io::Write;
        let mut f = std::fs::File::create(&out).unwrap();
        // ProgressQuestSaveData top-level hash 0x51dcd6fb
        if let Some((_, c)) = sf.fields.iter().find(|(h,_)| *h == 0x51dcd6fb) {
            writeln!(f, "# ProgressQuestSaveData fields: {}", c.fields.len()).unwrap();
            // dump each field name + if it's a _Flag array, the values
            for (i, fl) in c.fields.iter().enumerate() {
                let name = format!("field{}", i);
                match &fl.value {
                    FieldValue::Class(sub) => {
                        writeln!(f, "FIELD {} class", i).unwrap();
                        // dump all sub-fields; especially _Flag arrays
                        for (j, sf2) in sub.fields.iter().enumerate() {
                            match &sf2.value {
                                FieldValue::Array(a) => {
                                    let vals: Vec<String> = a.values.iter().map(|x| match x {
                                        FieldValue::U32(u) => u.to_string(),
                                        FieldValue::Enum(e) => e.as_i64().to_string(),
                                        _ => "-".into(),
                                    }).collect();
                                    writeln!(f, "  FIELD {} {} Array[{}] {}", i, j, a.values.len(), vals.join(",")).unwrap();
                                }
                                other => {
                                    writeln!(f, "  FIELD {} {} = {:?}", i, j, other).unwrap();
                                }
                            }
                        }
                    }
                    FieldValue::Array(a) => {
                        let vals: Vec<String> = a.values.iter().map(|x| match x {
                            FieldValue::U32(u) => u.to_string(),
                            FieldValue::Enum(e) => e.as_i64().to_string(),
                            _ => "-".into(),
                        }).collect();
                        writeln!(f, "FIELD {} Array[{}] {}", i, a.values.len(), vals.join(",")).unwrap();
                    }
                    other => {
                        writeln!(f, "FIELD {} = {:?}", i, other).unwrap();
                    }
                }
            }
        }
        println!("wrote quest structure to {}", out);
        return;
    }

    if cmd == "set-village" {
        if sys.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy set-village --sys <data00-1.bin> --idx <entry> --village <1=kamura,2=elgado> --steamid <id64> --out <out>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let mut sf = SaveFile::load(&sys, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        let li = sf.fields.iter_mut().find(|(h,_)| *h == 0x0b19f75b).map(|(_,c)| c)
            .expect("LoadInfoData not found");
        if let Some(FieldValue::Array(a)) = li.get_value_mut("_HunterArray") {
            if let Some(e) = a.get_as_class_mut(pc_slot) {
                if let Some(FieldValue::Enum(ev)) = e.get_value_mut("_CurrentVillageNo") {
                    *ev = ree_lib::save::types::EnumValue::E4(sw_slot as i32);
                    println!("set LoadInfo entry {} _CurrentVillageNo = {}", pc_slot, sw_slot);
                }
            }
        }
        sf.save(&out, &opts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); exit(1); });
        println!("wrote {}", out);
        return;
    }

    if cmd == "clear-slot" {
        // copies the EMPTY LoadInfo entry from switchsys (at sw_slot, default 1) into sys at pc_slot (wholesale)
        if sys.is_empty() || switchsys.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy clear-slot --sys <pc data00-1.bin> --switchsys <switch data00-1.bin> --pc-slot <idx> [--sw-slot <empty idx=1>] --steamid <id64> --out <out>");
            exit(1);
        }
        let mut popts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let mut pc = SaveFile::load(&sys, &mut popts).unwrap_or_else(|e| { eprintln!("load pc failed: {e}"); exit(1); });
        let mut sopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sw = SaveFile::load(&switchsys, &mut sopts).unwrap_or_else(|e| { eprintln!("load switch failed: {e}"); exit(1); });
        let sw_empty = if sw_slot > 0 { sw_slot } else { 1 };
        let src = sw.fields.iter().find(|(h,_)| *h == 0x0b19f75b).map(|(_,c)| c).expect("sw LoadInfoData not found");
        let src_cls = match src.get_value("_HunterArray") {
            Some(FieldValue::Array(a)) => a.get_as_class(sw_empty).expect("sw empty idx").clone(),
            _ => panic!("sw _HunterArray not array"),
        };
        let pc_li = pc.fields.iter_mut().find(|(h,_)| *h == 0x0b19f75b).map(|(_,c)| c).expect("pc LoadInfoData not found");
        let pc_arr = match pc_li.get_value_mut("_HunterArray") {
            Some(FieldValue::Array(a)) => a,
            _ => panic!("pc _HunterArray not array"),
        };
        if let Some(FieldValue::Class(bx)) = pc_arr.values.get_mut(pc_slot) {
            **bx = src_cls;
            println!("cleared pc LoadInfo entry {}", pc_slot);
        }
        pc.save(&out, &popts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); exit(1); });
        println!("wrote {}", out);
        return;
    }

    if cmd == "extract-npc" {
        if src.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy extract-npc --src <save> --steamid <id64> --out <file>");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        use std::io::Write;
        let mut f = std::fs::File::create(&out).unwrap();
        if let Some((_, c)) = sf.fields.iter().find(|(h,_)| *h == 0x1e32797d) {
            if let Some(FieldValue::Array(a)) = c.get_value("_NpcGuideChatUnlockFlag") {
                let vals: Vec<String> = a.values.iter().map(|x| match x {
                    FieldValue::U32(u) => u.to_string(),
                    FieldValue::Enum(e) => e.as_i64().to_string(),
                    _ => "-".into(),
                }).collect();
                writeln!(f, "{}", vals.join(",")).unwrap();
                println!("extracted {} NPC chat flags", a.values.len());
            }
        }
        return;
    }

    if cmd == "extract-flags" {
        if src.is_empty() || out.is_empty() {
            eprintln!("usage: save_copy extract-flags --src <save> --steamid <id64> --out <file>");
            exit(1);
        }
        let ctx = GameCtx::new(&AssetPaths::from_game(Game::MHRISE));
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        use std::io::Write;
        let mut f = std::fs::File::create(&out).unwrap();
        if let Some((_, c)) = sf.fields.iter().find(|(h,_)| *h == 0xf2fb669a) {
            if let Some(sd) = c.get_subclass("_Data1") {
                let ti = typeinfo_for(&ctx, sd);
                for (fi, fl) in sd.fields.iter().enumerate() {
                    let fname = ti.and_then(|t| t.fields.get(&fl.hash)).map(|x| x.name.clone())
                        .unwrap_or_else(|| format!("F{}", fi));
                    let _ = writeln!(f, "{}", fname);
                    dump_named(&mut f, &ctx, &fname, &fl.value);
                }
            }
        }
        println!("wrote flag reference to {}", out);
        return;
    }

    fn dump_named<W: Write>(f: &mut W, ctx: &GameCtx, path: &str, v: &FieldValue) {
        match v {
            FieldValue::Array(a) => {
                let is_class = a.values.iter().any(|x| matches!(x, FieldValue::Class(_)));
                if is_class {
                    for (i, e) in a.values.iter().enumerate() {
                        if let FieldValue::Class(c) = e {
                            let ti = typeinfo_for(ctx, c);
                            for (gi, g) in c.fields.iter().enumerate() {
                                let gname = ti.and_then(|t| t.fields.get(&g.hash)).map(|x| x.name.clone())
                                    .unwrap_or_else(|| format!("f{}", gi));
                                let p = format!("{}[{}].{}", path, i, gname);
                                let _ = writeln!(f, "{}", p);
                                dump_named(f, ctx, &p, &g.value);
                            }
                        }
                    }
                } else {
                    let vals: Vec<String> = a.values.iter().map(|x| match x {
                        FieldValue::U8(u) => u.to_string(),
                        FieldValue::U16(u) => u.to_string(),
                        FieldValue::U32(u) => u.to_string(),
                        FieldValue::U64(u) => u.to_string(),
                        FieldValue::S8(s) => s.to_string(),
                        FieldValue::S16(s) => s.to_string(),
                        FieldValue::S32(s) => s.to_string(),
                        FieldValue::S64(s) => s.to_string(),
                        FieldValue::Boolean(b) => if *b {"1"} else {"0"}.to_string(),
                        FieldValue::Enum(e) => e.as_i64().to_string(),
                        _ => "-".into(),
                    }).collect();
                    let _ = writeln!(f, "{}\tARRAY\t{}\t{}", path, a.values.len(), vals.join(","));
                }
            }
            FieldValue::Class(c) => {
                let ti = typeinfo_for(ctx, c);
                for (gi, g) in c.fields.iter().enumerate() {
                    let gname = ti.and_then(|t| t.fields.get(&g.hash)).map(|x| x.name.clone())
                        .unwrap_or_else(|| format!("f{}", gi));
                    let p = format!("{}.{}", path, gname);
                    let _ = writeln!(f, "{}", p);
                    dump_named(f, ctx, &p, &g.value);
                }
            }
            other => {
                let _ = writeln!(f, "{}\tVAL\t{:?}", path, other);
            }
        }
    }

    if cmd == "extract-class" {
        // --hash <top-level field hash> --prefix <name>
        if src.is_empty() || out.is_empty() || hash_top == 0 {
            eprintln!("usage: save_copy extract-class --src <save> --hash <top hash> --out <file> [--steamid]");
            exit(1);
        }
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        let sf = SaveFile::load(&src, &mut opts).unwrap_or_else(|e| { eprintln!("load failed: {e}"); exit(1); });
        use std::io::Write;
        let mut f = std::fs::File::create(&out).unwrap();
        if let Some((_, c)) = sf.fields.iter().find(|(h,_)| *h == hash_top) {
            if named_mode {
                let ctx = GameCtx::new(&AssetPaths::from_game(Game::MHRISE));
                let ti = typeinfo_for(&ctx, c);
                for (gi, g) in c.fields.iter().enumerate() {
                    let fname = ti.and_then(|t| t.fields.get(&g.hash)).map(|x| x.name.clone())
                        .unwrap_or_else(|| format!("R{}", gi));
                    let _ = writeln!(f, "{}", fname);
                    dump_named(&mut f, &ctx, &fname, &g.value);
                }
            } else {
                for (gi, g) in c.fields.iter().enumerate() {
                    let _ = writeln!(f, "{}", format!("R{}", gi));
                    dump_flag_value(&mut f, &format!("R{}", gi), &g.value);
                }
            }
        }
        println!("wrote class reference to {}", out);
        return;
    }

    fn dump_flag_value<W: Write>(f: &mut W, path: &str, v: &FieldValue) {
        match v {
            FieldValue::Array(a) => {
                // If array of classes (BitFlagTable), recurse into each; else dump values
                let is_class = a.values.iter().any(|x| matches!(x, FieldValue::Class(_)));
                if is_class {
                    for (i, e) in a.values.iter().enumerate() {
                        if let FieldValue::Class(c) = e {
                            for (gi, g) in c.fields.iter().enumerate() {
                                let _ = writeln!(f, "{}[{}].f{}", path, i, gi);
                                dump_flag_value(f, &format!("{}[{}].f{}", path, i, gi), &g.value);
                            }
                        }
                    }
                } else {
                    let vals: Vec<String> = a.values.iter().map(|x| match x {
                        FieldValue::U8(u) => u.to_string(),
                        FieldValue::U16(u) => u.to_string(),
                        FieldValue::U32(u) => u.to_string(),
                        FieldValue::U64(u) => u.to_string(),
                        FieldValue::S8(s) => s.to_string(),
                        FieldValue::S16(s) => s.to_string(),
                        FieldValue::S32(s) => s.to_string(),
                        FieldValue::S64(s) => s.to_string(),
                        FieldValue::Boolean(b) => if *b {"1"} else {"0"}.to_string(),
                        FieldValue::Enum(e) => e.as_i64().to_string(),
                        _ => "-".into(),
                    }).collect();
                    let _ = writeln!(f, "{}\tARRAY\t{}\t{}", path, a.values.len(), vals.join(","));
                }
            }
            FieldValue::Class(c) => {
                for (gi, g) in c.fields.iter().enumerate() {
                    let _ = writeln!(f, "{}.f{}", path, gi);
                    dump_flag_value(f, &format!("{}.f{}", path, gi), &g.value);
                }
            }
            other => {
                let _ = writeln!(f, "{}\tVAL\t{:?}", path, other);
            }
        }
    }

    if cmd == "convert" {
        if src.is_empty() || out.is_empty() || steamid == 0 {
            eprintln!("usage: save_copy convert --src <switch save> --steamid <id64> --out <outfile>");
            exit(1);
        }
        let ctx = GameCtx::new(&AssetPaths::from_game(Game::MHRISE));
        let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
        match SaveFile::load(&src, &mut opts) {
            Ok(mut sf) => {
                println!("Loaded {}: flags={:?}, curve_index={:?}", src, sf.flags, opts.curve_index);
                sf.flags = SaveFlags::CITRUS;
                sf.save(&out, &opts).unwrap_or_else(|e| { eprintln!("save failed: {e}"); exit(1); });
                let data = std::fs::read(&out).unwrap_or_else(|e| { eprintln!("read failed: {e}"); exit(1); });
                println!("Wrote {} ({} bytes)", out, data.len());
                println!("  header flags bytes: {:02x} {:02x} {:02x} {:02x}", data[8], data[9], data[10], data[11]);
                // verify by reloading
                let mut vopts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
                match SaveFile::load(&out, &mut vopts) {
                    Ok(vf) => println!("Verify reload OK: flags={:?}, {} top-level fields", vf.flags, vf.fields.len()),
                    Err(e) => eprintln!("Verify reload FAILED: {e}"),
                }
            }
            Err(e) => { eprintln!("failed to load source save: {e}"); exit(1); }
        }
        return;
    }

    if cmd.is_empty() || save.is_empty() || steamid == 0 {
        eprintln!("usage: save_copy dump --save <file> --steamid <id64> [--depth N] [--top N]");
        exit(1);
    }

    let ctx = GameCtx::new(&AssetPaths::from_game(Game::MHRISE));

    let mut opts = SaveOptions::new(Game::MHRISE).id(steamid).curve_index(107);
    match SaveFile::load(&save, &mut opts) {
        Ok(sf) => {
            println!("== top-level fields in {} ==", save);
            for (i, (hash, class)) in sf.fields.iter().enumerate() {
                let name = typeinfo_for(&ctx, class)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| format!("<cls 0x{:08x}>", class.hash));
                println!("[{i}] field_hash=0x{hash:08x} -> {name} (cls 0x{:08x})", class.hash);
            }
            // dump top-level classes shallowly to find the character slots
            if let Some(t) = top {
                if let Some((_, class)) = sf.fields.get(t) {
                    println!("\n== top-level [{t}] deep ==");
                    dump_class(&ctx, class, 0, depth);
                }
            } else {
                for (i, (_, class)) in sf.fields.iter().enumerate() {
                    println!("\n== top-level [{i}] ==");
                    dump_class(&ctx, class, 0, 1);
                }
            }
        }
        Err(e) => {
            eprintln!("failed to load save: {e}");
            exit(1);
        }
    }
}
