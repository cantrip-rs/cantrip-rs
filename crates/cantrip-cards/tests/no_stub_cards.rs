//! A card file that is nothing but `CardDef { ..oracle::X }` is a stub: it adds no behavior, so it is right only for a
//! card whose Oracle text is core keywords, and such a card needs no file at all (`cantrip-oracle`'s `PLAIN`). A stub
//! for any other card would register it as ported while it does nothing, and every scenario using it would pass or fail
//! for the wrong reason. This test fails on one.
//!
//! The documented exceptions, each of them in the card's own file or data:
//! - a draft-only card (`"draftAbilities"` in its JSON, such as Cogwork Librarian) has no game rules to port;
//! - a card whose every line is a characteristic the generator already reads from its JSON (Hogaak's convoke, delve and
//!   "can't spend mana") says so in its file, in a comment that starts `// Characteristics only:`.

use std::fs;
use std::path::{Path, PathBuf};

/// The Oracle words of the core keywords a card can have and still need no file of its own (the same list as
/// `cantrip-oracle/build.rs`'s `is_plain`).
const KEYWORD_WORDS: [&str; 33] = [
    "flying", "haste", "vigilance", "trample", "first strike", "double strike", "deathtouch", "lifelink", "reach", "defender",
    "flash", "hexproof", "indestructible", "menace", "infect", "shadow", "shroud", "changeling", "decayed", "islandwalk", "plainswalk", "swampwalk", "mountainwalk", "forestwalk", "banding",
    "horsemanship", "fear", "intimidate", "skulk", "nonbasic landwalk", "legendary landwalk", "wither", "phasing",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Whether the file's code, comments aside, is `pub static CARD: CardDef = CardDef { ..oracle::X };` and nothing more.
fn is_bare_stub(source: &str) -> bool {
    let code: String = source
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .filter(|l| !l.trim_start().starts_with("use "))
        .collect::<Vec<_>>()
        .join(" ");
    let squeezed: String = code.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(rest) = squeezed.strip_prefix("pub static CARD: CardDef = CardDef { ..oracle::") else { return false };
    let Some(name) = rest.strip_suffix(" };") else { return false };
    !name.is_empty() && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Whether every line of the Oracle text, reminder text left out, is core keywords.
fn only_core_keywords(oracle: &str) -> bool {
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
    text.split('\n')
        .filter(|l| !l.trim().is_empty())
        .all(|line| line.split(',').all(|part| KEYWORD_WORDS.contains(&part.trim().to_lowercase().as_str())))
}

/// The content JSON of the card whose file is `stem` (`scale_up` is `scale-up.json`), in whichever dataset has it.
fn card_json(stem: &str) -> Option<serde_json::Value> {
    let file = format!("{}.json", stem.replace('_', "-"));
    for dataset in fs::read_dir(root().join("content")).ok()?.flatten() {
        let path = dataset.path().join("cards").join(&file);
        if let Ok(text) = fs::read_to_string(&path) {
            return serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(&text)).ok();
        }
    }
    None
}

#[test]
fn no_card_file_is_a_stub_for_a_card_that_has_rules_text() {
    let mut stubs = Vec::new();
    let mut seen = 0;
    for entry in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards")).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
        seen += 1;
        if !is_bare_stub(&source) {
            continue;
        }
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        let json = card_json(&stem).unwrap_or_else(|| panic!("{stem}.rs has no card in content/*/cards"));
        let oracle = json.get("oracleText").and_then(|t| t.as_str()).unwrap_or("");
        let draft_only = json.get("draftAbilities").is_some();
        let characteristics_only = source.lines().any(|l| l.trim_start().starts_with("// Characteristics only:"));
        if !only_core_keywords(oracle) && !draft_only && !characteristics_only {
            stubs.push(stem);
        }
    }
    assert!(seen > 500, "found only {seen} card files: is src/cards where the test looks?");
    assert!(
        stubs.is_empty(),
        "these card files are bare `CardDef {{ ..oracle::X }}` stubs for cards whose Oracle text is more than core keywords; \
         port the card (docs/adding-cards.md), or, if its text is all characteristics, say so with a `// Characteristics only:` \
         comment: {stubs:?}"
    );
}

#[test]
fn the_stub_test_recognises_a_stub() {
    assert!(is_bare_stub("use cantrip_api::*;\nuse cantrip_oracle as oracle;\n\npub static CARD: CardDef = CardDef { ..oracle::SCALE_UP };\n"));
    assert!(!is_bare_stub("pub static CARD: CardDef = CardDef { spell: None, ..oracle::SCALE_UP };"));
    assert!(only_core_keywords("Flying, lifelink\nTrample (It can be blocked.)"));
    assert!(!only_core_keywords("Flying\nWhenever this attacks, draw a card."));
}
