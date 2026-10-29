//! Generates every card's characteristics from `content/<dataset>/cards/*.json` into `$OUT_DIR/generated.rs`: one
//! `CardDef` constant per card, its characteristics and nothing else (name, cost, colors, types, P/T, loyalty, keywords
//! that are core keywords, Oracle text), and `PLAIN`, the cards that need no file of their own. A card's file in the
//! cards crate adds its behavior with struct update syntax: `CardDef { spell: ..., ..oracle::LIGHTNING_BOLT }`.
//!
//! Adding a card is adding its JSON; nothing is checked in that this writes. Every subtype a card prints must be in
//! `cantrip-state/src/subtypes.rs` (rules data, CR 205.3), or the build fails naming the card and the subtype.

use std::collections::HashSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

type Card = Map<String, Value>;

const TYPES: [&str; 9] = ["Artifact", "Creature", "Enchantment", "Instant", "Land", "Planeswalker", "Sorcery", "Battle", "Kindred"];
const BASIC_LAND: [(&str, &str); 5] = [("Plains", "W"), ("Island", "U"), ("Swamp", "B"), ("Mountain", "R"), ("Forest", "G")];
const KEYWORDS: [&str; 43] = [
    "Flying", "Haste", "Vigilance", "Trample", "FirstStrike", "DoubleStrike", "Deathtouch", "Lifelink", "Reach", "Defender",
    "Flash", "Hexproof", "Indestructible", "Menace", "Infect", "Shadow", "Shroud", "CantBeBlocked", "CantBeCountered",
    "CantSpendManaToCast", "CastFromYourGraveyard", "Daybound", "Nightbound", "Changeling", "Decayed",
    "CantBeBlockedByMoreThanOne", "Islandwalk", "Plainswalk", "Swampwalk", "Mountainwalk", "Forestwalk", "Banding",
    "CrewsAsThoughPowerTwoGreater", "SaddlesAsThoughPowerTwoGreater", "Horsemanship", "Fear", "Intimidate", "Skulk",
    "NonbasicLandwalk", "LegendaryLandwalk", "Wither", "Phasing",
    "CantBeCopied",
];
const COLORS: &str = "WUBRG";
const COLOR_NAMES: [&str; 5] = ["White", "Blue", "Black", "Red", "Green"];
/// The Oracle words of the core keywords a card can have and still need no file of its own.
const KEYWORD_WORDS: [&str; 33] = [
    "flying", "haste", "vigilance", "trample", "first strike", "double strike", "deathtouch", "lifelink", "reach", "defender",
    "flash", "hexproof", "indestructible", "menace", "infect", "shadow", "shroud", "changeling", "decayed", "islandwalk", "plainswalk", "swampwalk", "mountainwalk", "forestwalk", "banding",
    "horsemanship", "fear", "intimidate", "skulk", "nonbasic landwalk", "legendary landwalk", "wither", "phasing",
];

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let content = manifest.join("../../content");
    let subtypes_rs = manifest.join("../cantrip-state/src/subtypes.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", subtypes_rs.display());
    let known = known_subtypes(&subtypes_rs);

    let mut cards: Vec<(Card, String)> = Vec::new(); // the card, and the file it came from
    let mut datasets: Vec<PathBuf> = std::fs::read_dir(&content).unwrap().map(|e| e.unwrap().path()).collect();
    datasets.sort();
    for dataset in datasets {
        let folder = dataset.join("cards");
        if !folder.is_dir() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", folder.display());
        let mut files: Vec<PathBuf> = std::fs::read_dir(&folder).unwrap().map(|e| e.unwrap().path()).collect();
        files.sort();
        for file in files {
            if file.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            println!("cargo:rerun-if-changed={}", file.display());
            let source = file.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&file).unwrap();
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
            let mut card: Card = match serde_json::from_str(text) {
                Ok(Value::Object(m)) => m,
                Ok(_) => panic!("{source}: a card is a JSON object"),
                Err(e) => panic!("{source}: {e}"),
            };
            if let Some(halves) = card.get("halves").and_then(Value::as_array).filter(|h| h.len() == 2).cloned() {
                // A split card (CR 709): each half is a constant of its own, which the card's file points `split:` at,
                // and the card is both halves at once in every zone but the stack (CR 709.4): their costs added, their
                // types together.
                let halves: Vec<&Card> = halves.iter().map(|h| h.as_object().expect("a half is an object")).collect();
                for half in &halves {
                    let mut part = pick(half, &["name", "manaCost", "type", "oracleText"]);
                    part.insert("part".into(), Value::Bool(true));
                    if half.get("aftermath") == Some(&Value::Bool(true)) {
                        part.insert("aftermath".into(), Value::Bool(true));
                    }
                    cards.push((part, source.clone()));
                }
                // The types and subtypes of both halves (CR 709.4c, 709.5a): a Room's shared type line is on each.
                let mut words: Vec<String> = Vec::new();
                let mut subtypes: Vec<String> = Vec::new();
                for h in &halves {
                    let (types, subs) = split_type(&str_of(h, "type"));
                    for w in types {
                        if !words.contains(&w) {
                            words.push(w);
                        }
                    }
                    for w in subs {
                        if !subtypes.contains(&w) {
                            subtypes.push(w);
                        }
                    }
                }
                let subtypes_text = if subtypes.is_empty() { String::new() } else { format!(" \u{2014} {}", subtypes.join(" ")) };
                let mut combined = Card::new();
                combined.insert("name".into(), card["name"].clone());
                combined.insert("manaCost".into(), Value::String(halves.iter().map(|h| str_of(h, "manaCost")).collect()));
                combined.insert("type".into(), Value::String(format!("{}{}", words.join(" "), subtypes_text)));
                combined.insert("oracleText".into(), Value::String(halves.iter().map(|h| str_of(h, "oracleText")).collect::<Vec<_>>().join("\n//\n")));
                combined.insert("split".into(), Value::Bool(true));
                card = combined;
            }
            // An Adventure (CR 715) or an Omen (CR 720) is a second set of characteristics: its own constant, which the
            // card's file points `adventure:` or `omen:` at.
            let insets: Vec<Card> = ["adventure", "omen"]
                .iter()
                .filter_map(|key| card.get(*key).and_then(Value::as_object))
                .map(|a| {
                    let mut part = pick(a, &["name", "manaCost", "type", "oracleText"]);
                    part.insert("part".into(), Value::Bool(true));
                    part
                })
                .collect();
            cards.push((card, source.clone()));
            for part in insets {
                cards.push((part, source.clone()));
            }
        }
    }

    let mut out = String::new();
    out.push_str("// Characteristics of every card in content/, generated by crates/cantrip-oracle/build.rs.\n\nuse cantrip_api::*;\n\n");
    let mut names: HashSet<String> = HashSet::new();
    let mut plain: Vec<String> = Vec::new();
    // A station card's striation [P/T] box (CR 721.2b): printed, but no characteristic of the card in any zone (CR 721.2c).
    let mut station: Vec<(String, String)> = Vec::new();
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by(|&a, &b| str_of(&cards[a].0, "name").cmp(&str_of(&cards[b].0, "name")));
    for i in order {
        let (card, source) = &cards[i];
        let name = str_of(card, "name");
        let konst = ident(&name);
        if !names.insert(konst.clone()) {
            continue;
        }
        let fields = match face_fields(card, false, source, &known) {
            Ok(f) => f,
            Err(e) => {
                println!("cargo:warning=skipped {name}: {e}");
                continue;
            }
        };
        if is_plain(card) {
            plain.push(konst.clone());
        }
        if let Some(pt) = card.get("stationPt").and_then(Value::as_str) {
            let valid = pt.split_once('/').is_some_and(|(p, t)| [p, t].iter().all(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit() || c == '*')));
            assert!(valid, "{source}: stationPt is the box as printed, \"2/2\"; not {pt:?}");
            station.push((name.clone(), pt.to_string()));
        }
        // The back face of a double-faced card (CR 711.1) is a constant of its own, which the front refers to.
        if let Some(back) = card.get("backFace").and_then(Value::as_object) {
            let back_const = ident(&str_of(back, "name"));
            if names.insert(back_const.clone()) {
                let back_fields = face_fields(back, true, source, &known).unwrap_or_else(|e| panic!("{source}: back face: {e}"));
                emit(&mut out, &back_const, &back_fields);
            }
        }
        emit(&mut out, &konst, &fields);
    }
    out.push_str("/// The cards that are their characteristics and nothing more: keywords the core plays, a basic land type's mana\n");
    out.push_str("/// (CR 305.6). Registered as they are unless a card file says more.\npub static PLAIN: &[&CardDef] = &[\n");
    for c in &plain {
        writeln!(out, "    &{c},").unwrap();
    }
    out.push_str("];\n");
    out.push_str("\n/// The power and toughness box each station card prints in its striation (CR 721.2b), by name, for a client to draw\n");
    out.push_str("/// there. Presentation only: the striation's static ability gives the permanent that base power and toughness, and the\n");
    out.push_str("/// card has none in any other zone (CR 721.2c).\npub static STATION_PT: &[(&str, &str)] = &[\n");
    for (name, pt) in &station {
        writeln!(out, "    ({}, {}),", rust_str(name), rust_str(pt)).unwrap();
    }
    out.push_str("];\n\n/// The [P/T] box printed in the striation of the station card named `name`, \"2/2\"; none for any other card.\n");
    out.push_str("pub fn station_pt(name: &str) -> Option<&'static str> {\n");
    out.push_str("    STATION_PT.binary_search_by(|(n, _)| (*n).cmp(name)).ok().map(|i| STATION_PT[i].1)\n}\n");
    std::fs::write(Path::new(&std::env::var("OUT_DIR").unwrap()).join("generated.rs"), out).unwrap();
}

fn emit(out: &mut String, konst: &str, fields: &[String]) {
    writeln!(out, "pub const {konst}: CardDef = CardDef {{").unwrap();
    out.push_str("    oracle: true,\n");
    for f in fields {
        writeln!(out, "{f}").unwrap();
    }
    out.push_str("    ..CardDef::BLANK\n};\n\n");
}

fn pick(from: &Card, keys: &[&str]) -> Card {
    keys.iter().filter_map(|k| from.get(*k).map(|v| (k.to_string(), v.clone()))).collect()
}

fn str_of(card: &Card, key: &str) -> String {
    card.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

/// The names the `Subtype` enum has, read from its `name()` arms.
fn known_subtypes(path: &Path) -> HashSet<String> {
    let text = std::fs::read_to_string(path).unwrap();
    let mut known = HashSet::new();
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("Subtype::") {
            if let Some((_, name)) = rest.split_once(" => \"") {
                if let Some(name) = name.strip_suffix("\",") {
                    known.insert(name.to_string());
                }
            }
        }
    }
    assert!(!known.is_empty(), "no subtypes read from {}", path.display());
    known
}

fn ident(name: &str) -> String {
    let mut s = String::new();
    let mut pending = false;
    for c in name.replace('\'', "").chars() {
        if c.is_ascii_alphanumeric() {
            if pending && !s.is_empty() {
                s.push('_');
            }
            pending = false;
            s.push(c.to_ascii_uppercase());
        } else {
            pending = true;
        }
    }
    if s.starts_with(|c: char| c.is_ascii_digit()) { format!("_{s}") } else { s }
}

fn subtype_ident(name: &str) -> String {
    name.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

fn rust_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => write!(out, "\\u{{{:04x}}}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn cost(text: &str) -> Result<String, String> {
    let (mut generic, mut x) = (0u32, 0u32);
    let mut symbols = [0u32; 6];
    let mut hybrid: Vec<(char, char)> = Vec::new();
    let mut phyrexian = [0u32; 5];
    let mut twobrid = [0u32; 5];
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else { break };
        let sym = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        let cs: Vec<char> = sym.chars().collect();
        let color = |c: char| COLORS.find(c);
        if !sym.is_empty() && sym.chars().all(|c| c.is_ascii_digit()) {
            generic += sym.parse::<u32>().unwrap();
        } else if sym == "X" {
            x += 1;
        } else if cs.len() == 1 && color(cs[0]).is_some() {
            symbols[color(cs[0]).unwrap()] += 1;
        } else if sym == "C" {
            symbols[5] += 1;
        } else if cs.len() == 3 && color(cs[0]).is_some() && cs[1] == '/' && cs[2] == 'P' {
            phyrexian[color(cs[0]).unwrap()] += 1;
        } else if cs.len() == 3 && cs[0] == '2' && cs[1] == '/' && color(cs[2]).is_some() {
            // A monocolored hybrid symbol, {2/W} (CR 107.4e).
            twobrid[color(cs[2]).unwrap()] += 1;
        } else if cs.len() == 3 && (color(cs[0]).is_some() || cs[0] == 'C') && cs[1] == '/' && color(cs[2]).is_some() {
            // A hybrid symbol of two colors, {W/U}, or of colorless and a color, {C/W} (CR 107.4e).
            hybrid.push((cs[0], cs[2]));
        } else {
            return Err(format!("mana symbol {{{sym}}}"));
        }
    }
    if hybrid.len() > 6 {
        return Err("more than six hybrid symbols".into());
    }
    let mut slots: Vec<String> = hybrid.iter().map(|(a, b)| format!("Some((Mana::{a}, Mana::{b}))")).collect();
    slots.resize(6, "None".into());
    Ok(format!(
        "ManaCost {{ generic: {generic}, symbols: {symbols:?}, hybrid: [{}], twobrid: {twobrid:?}, phyrexian: {phyrexian:?}, x: {x} }}",
        slots.join(", ")
    ))
}

/// A type line split at its first dash into the card types and supertypes, and the subtypes.
fn split_type(line: &str) -> (Vec<String>, Vec<String>) {
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let mut split = None;
    for (n, &(at, c)) in chars.iter().enumerate() {
        if !c.is_whitespace() {
            continue;
        }
        let mut j = n;
        while j < chars.len() && chars[j].1.is_whitespace() {
            j += 1;
        }
        if j + 1 < chars.len() && (chars[j].1 == '\u{2014}' || chars[j].1 == '-') && chars[j + 1].1.is_whitespace() {
            let mut k = j + 1;
            while k < chars.len() && chars[k].1.is_whitespace() {
                k += 1;
            }
            split = Some((at, chars.get(k).map_or(line.len(), |c| c.0)));
            break;
        }
    }
    let words = |s: &str| s.split_whitespace().map(str::to_string).collect::<Vec<_>>();
    match split {
        Some((a, b)) => (words(&line[..a]), words(&line[b..])),
        None => (words(line), Vec::new()),
    }
}

fn is_plain(card: &Card) -> bool {
    if card.get("part") == Some(&Value::Bool(true)) {
        return true;
    }
    if ["backFace", "adventure", "omen", "prepare", "halves", "split", "prototype"].iter().any(|k| card.contains_key(*k)) {
        return false;
    }
    without_reminder(&str_of(card, "oracleText")).split('\n').filter(|l| !l.trim().is_empty()).all(|line| {
        line.split([',', ';']).all(|part| {
            let part = part.trim().to_lowercase();
            KEYWORD_WORDS.contains(&part.as_str()) || toxic_n(&part).is_some()
        })
    })
}

/// Oracle text without its reminder text (CR 207.2a), the parenthesized parts.
fn without_reminder(oracle: &str) -> String {
    let mut text = String::new();
    let mut in_reminder = false;
    for c in oracle.chars() {
        match c {
            '(' if !in_reminder => in_reminder = true,
            ')' if in_reminder => in_reminder = false,
            _ if in_reminder => {}
            c => text.push(c),
        }
    }
    text
}

/// The toxic N its Oracle text prints, added up (CR 702.164b): each keyword part of a line - alone ("Toxic 2") or in a
/// keyword list ("Menace, toxic 1") - that is "toxic N".
fn printed_toxic(card: &Card) -> u16 {
    let text = without_reminder(&str_of(card, "oracleText"));
    text.split('\n').flat_map(|line| line.split([',', ';'])).filter_map(|part| toxic_n(&part.to_lowercase())).sum()
}

/// The N of a toxic N (CR 702.164a), as content's `keywords` ("Toxic 2") or Oracle text ("toxic 2") words it.
fn toxic_n(word: &str) -> Option<u16> {
    let word = word.trim();
    word.strip_prefix("Toxic ").or_else(|| word.strip_prefix("toxic "))?.parse().ok()
}

/// The `CardDef` fields of one face: a card, or the back face of a double-faced card (CR 711).
fn face_fields(card: &Card, is_back: bool, source: &str, known: &HashSet<String>) -> Result<Vec<String>, String> {
    let name = str_of(card, "name");
    let mana = match card.get("manaCost").and_then(Value::as_str) {
        Some(t) => Some(cost(t)?),
        None => None,
    };
    let (words, subs) = split_type(&str_of(card, "type"));
    for s in &subs {
        if !known.contains(s) {
            panic!("{source}: {name} has the subtype {s}, which is not in cantrip-state/src/subtypes.rs (CR 205.3); add it there");
        }
    }
    let types: Vec<String> = words.iter().filter(|w| TYPES.contains(&w.as_str())).map(|t| format!("CardType::{t}")).collect();
    let has = |w: &str| words.iter().any(|x| x == w);
    let keywords: Vec<String> = card
        .get("keywords")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|k| KEYWORDS.contains(k))
        .map(|k| format!("Keyword::{k}"))
        .collect();
    let mut fields = vec![format!("    name: {},", rust_str(&name))];
    match mana {
        Some(m) => fields.push(format!("    cost: {m},")),
        None => fields.push("    has_cost: false,".into()),
    }
    // A color indicator (CR 204) defines the colors of a face; otherwise they follow its mana cost.
    let indicator: Vec<usize> = card
        .get("colorIndicator")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|c| COLOR_NAMES.iter().position(|n| *n == c))
        .collect();
    if !indicator.is_empty() {
        fields.push(format!("    colors: Some(ColorSet({})),", indicator.iter().fold(0u32, |a, i| a | 1 << i)));
    }
    fields.push(format!("    types: TypeSet::of(&[{}]),", types.join(", ")));
    let (basic, legendary, snow) = (has("Basic"), has("Legendary"), has("Snow"));
    if basic || legendary || snow {
        fields.push(format!("    supertypes: Supertypes {{ basic: {basic}, legendary: {legendary}, snow: {snow} }},"));
    }
    if !subs.is_empty() {
        let list: Vec<String> = subs.iter().map(|s| format!("Subtype::{}", subtype_ident(s))).collect();
        fields.push(format!("    subtypes: SubtypeSet::of(&[{}]),", list.join(", ")));
    }
    for key in ["power", "toughness", "loyalty", "defense"] {
        match card.get(key) {
            None | Some(Value::Null) => {}
            Some(Value::Number(n)) if n.is_i64() => fields.push(format!("    {key}: Some({}),", n.as_i64().unwrap())),
            Some(Value::String(s)) if !s.trim_start_matches('-').is_empty() && s.trim_start_matches('-').chars().all(|c| c.is_ascii_digit()) => {
                fields.push(format!("    {key}: Some({}),", s.parse::<i64>().unwrap()))
            }
            Some(_) => fields.push(format!("    {key}: Some(0),")),
        }
    }
    if !keywords.is_empty() {
        fields.push(format!("    keywords: Keywords::of(&[{}]),", keywords.join(", ")));
    }
    // Toxic N (CR 702.164a), "Toxic 2" in `keywords`: each instance's N, added up (CR 702.164b).
    let toxic: u16 = card
        .get("keywords")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(toxic_n)
        .sum();
    // A content file that leaves out a toxic its text prints, or gives another N, would build the card without it.
    let printed = printed_toxic(card);
    if toxic != printed {
        panic!(
            "{source}: {name}'s Oracle text prints toxic {printed} in all, but its `keywords` give toxic {toxic}: list \"Toxic N\" \
             in `keywords` for each toxic N the text prints (CR 702.164a)"
        );
    }
    if toxic > 0 {
        fields.push(format!("    toxic: {toxic},"));
    }
    // Convoke, delve and improvise (CR 702.51, 702.66, 702.126): what else may pay the total cost.
    let flag = |k: &str| card.get(k) == Some(&Value::Bool(true));
    // And assist (CR 702.132a): another player may pay some of its generic mana.
    if flag("convoke") || flag("delve") || flag("improvise") || flag("assist") {
        fields.push(format!(
            "    pay_with: PayWith {{ convoke: {}, delve: {}, improvise: {}, assist: {} }},",
            flag("convoke"),
            flag("delve"),
            flag("improvise"),
            flag("assist")
        ));
    }
    if flag("aftermath") {
        fields.push("    aftermath: true,".into());
    }
    // Prototype (CR 702.160a, 718.1): the inset frame's mana cost, power and toughness, `"prototype": {"manaCost": "{2}{R}",
    // "power": 3, "toughness": 2}`. Its keyword line must be the Oracle text's, so the two can't disagree.
    if let Some(prototype) = card.get("prototype") {
        let p = prototype.as_object().unwrap_or_else(|| panic!("{source}: prototype is an object: {{\"manaCost\", \"power\", \"toughness\"}}"));
        let mana = p.get("manaCost").and_then(Value::as_str).unwrap_or_else(|| panic!("{source}: prototype has a manaCost"));
        let stat = |key: &str| match p.get(key) {
            Some(Value::Number(n)) if n.is_i64() => n.as_i64().unwrap(),
            _ => panic!("{source}: prototype's {key} is a number"),
        };
        let (power, toughness) = (stat("power"), stat("toughness"));
        let line = format!("Prototype {mana} \u{2014} {power}/{toughness}");
        let printed = without_reminder(&str_of(card, "oracleText"));
        if !printed.lines().any(|l| l.trim() == line) {
            panic!("{source}: {name}'s Oracle text has no line \"{line}\" for its prototype block (CR 718.1)");
        }
        fields.push(format!("    prototype: Some(Prototype {{ cost: {}, power: {power}, toughness: {toughness}, text: {} }}),", cost(mana)?, rust_str(&line)));
    }
    if !is_back {
        if let Some(back) = card.get("backFace").and_then(Value::as_object) {
            fields.push(format!("    back: Some(&{}),", ident(&str_of(back, "name"))));
            if flag("backFacePlayableAsLand") {
                fields.push("    back_land: true,".into());
            }
        }
    }
    let basics: Vec<String> = subs.iter().filter_map(|s| BASIC_LAND.iter().find(|(n, _)| n == s)).map(|(_, m)| format!("Mana::{m}")).collect();
    if !basics.is_empty() {
        fields.push(format!("    basic_land_types: &[{}],", basics.join(", ")));
    }
    fields.push(format!("    text: {},", rust_str(&str_of(card, "oracleText"))));
    Ok(fields)
}
