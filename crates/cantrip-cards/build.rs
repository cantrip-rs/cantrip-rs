//! The card registry, generated from the files in `src/cards`: one file per card, each with `pub static CARD`, and
//! optionally `pub static EXTRA: &[&CardDef]` for definitions it brings with it (tokens, a copy's exception, the
//! basics). Adding a card is adding its file, and nothing else.

use std::fmt::Write;

/// A card file's module name: `r#name`, with a leading `_` where the name starts with a digit (70,000 Light-Years from
/// Home), as the oracle crate names its constant.
fn module(name: &str) -> String {
    if name.starts_with(|c: char| c.is_ascii_digit()) { format!("_{name}") } else { format!("r#{name}") }
}

fn main() {
    let dir = std::path::Path::new("src/cards");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("src/cards")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs") && p.file_stem().is_some_and(|s| s != "mod"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().to_string())
        .collect();
    names.sort();
    // Keep module paths usable by IDEs too: canonicalize() adds Windows' verbatim (\\?\) prefix.
    let manifest_dir = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let mut out = String::new();
    for name in &names {
        let path = dir.join(format!("{name}.rs"));
        println!("cargo:rerun-if-changed={}", path.display());
        let m = module(name);
        let module_path = manifest_dir.join(&path).to_string_lossy().replace('\\', "/");
        writeln!(out, "#[path = {module_path:?}]\nmod {m};").unwrap();
    }
    out.push_str("\n/// Every card definition the crate has.\npub fn all() -> Vec<&'static CardDef> {\n    let mut cards: Vec<&'static CardDef> = Vec::new();\n");
    for name in &names {
        let text = std::fs::read_to_string(dir.join(format!("{name}.rs"))).unwrap();
        let m = module(name);
        if text.contains("pub static CARD:") {
            writeln!(out, "    cards.push(&{m}::CARD);").unwrap();
        }
        if text.contains("pub static EXTRA:") {
            writeln!(out, "    cards.extend({m}::EXTRA.iter().copied());").unwrap();
        }
    }
    // Cards that are their characteristics alone; a card's own file, pushed first, wins over its entry there.
    // Only a card's name is a card's: a token named for its subtypes (CR 111.4) never stands for a card.
    out.push_str("    let named: std::collections::HashSet<&str> = cards.iter().filter(|c| c.oracle).map(|c| c.name).collect();\n");
    out.push_str("    cards.extend(cantrip_oracle::PLAIN.iter().copied().filter(|p| !named.contains(p.name)));\n");
    out.push_str("    cards\n}\n");
    // Each card file's card with the definitions it brings, for the tests that hold a token to the text that makes it.
    out.push_str("\n/// Each card file's `CARD`, with its `EXTRA` (empty when it has none).\npub fn files() -> Vec<(&'static CardDef, &'static [&'static CardDef])> {\n    vec![\n");
    for name in &names {
        let text = std::fs::read_to_string(dir.join(format!("{name}.rs"))).unwrap();
        let m = module(name);
        if text.contains("pub static CARD:") {
            let extra = if text.contains("pub static EXTRA:") { format!("{m}::EXTRA") } else { "&[]".to_string() };
            writeln!(out, "        (&{m}::CARD, {extra}),").unwrap();
        }
    }
    out.push_str("    ]\n}\n");
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("cards.rs"), out).unwrap();
}
