//! A token's name is set by the text that creates it (CR 111.4): the name the text gives it ("a ... token named Moloid",
//! "create Boo, a legendary ... token"), or else its subtypes followed by "Token" ("Phyrexian Hydra Token"). A definition
//! the text doesn't name is built with `token!`, which derives the name from the subtypes it writes in the text's order
//! (`CardDef::token_words`), and one it does name writes that name. Each token a card file brings (its `EXTRA`) is held
//! here to the Oracle text of the card that makes it: a derived name to the words "[subtypes] ... token" there, not
//! followed by "named", and a written name to the text naming it. A token named by a definition of the rules (CR 111.10:
//! the Roles, the Walker, an Incubator's Phyrexian Token) is held to the rule's words instead.

use cantrip_api::*;

/// The tokens CR 111.10 names, and the words a card's text makes each one with: the Role tokens by their Role (CR
/// 111.10j-r), the Walker (CR 111.10d), and the Phyrexian Token an Incubator transforms into (CR 111.10i).
const PREDEFINED_NAMED: [(&str, &str); 9] = [
    ("Cursed", "Cursed Role token"),
    ("Monster", "Monster Role token"),
    ("Royal", "Royal Role token"),
    ("Sorcerer", "Sorcerer Role token"),
    ("Virtuous", "Virtuous Role token"),
    ("Wicked", "Wicked Role token"),
    ("Young Hero", "Young Hero Role token"),
    ("Walker", "Walker token"),
    ("Phyrexian Token", "Incubator token"),
];

/// The words a type line's card types and supertypes are, which may come between a token's subtypes and "token".
const TYPE_WORDS: [&str; 9] = ["artifact", "creature", "enchantment", "land", "planeswalker", "battle", "legendary", "snow", "kindred"];

/// The keyword actions whose rules make a token (CR 701), as the card's text says them, and the token as the rule says it:
/// empower Jace (CR 701.71a), amass (CR 701.47a, Orcs and Goblins) and investigate (CR 701.16a).
const KEYWORD_TOKENS: [(&str, &str); 4] = [
    ("empower jace", "a blue Jace planeswalker token"),
    ("amass orcs", "a 0/0 black Orc Army creature token"),
    ("amass goblins", "a 0/0 black Goblin Army creature token"),
    ("investigate", "a Clue token"),
];

/// The Oracle text of a card and every face of it a token could be made by, with the text of what it brings that is no
/// token (a prepare spell, a dungeon's rooms), and the tokens its keyword actions make in the rules' words.
fn texts(card: &'static CardDef, extras: &[&'static CardDef]) -> String {
    let mut all: Vec<&str> = vec![card.text];
    all.extend(card.back.map(|b| b.text));
    all.extend(card.adventure.map(|a| a.text));
    all.extend(card.omen.map(|o| o.text));
    all.extend(card.prepare.map(|p| p.text));
    if let Some((left, right)) = card.split {
        all.extend([left.text, right.text]);
    }
    for def in extras.iter().chain([&card]).filter(|def| !is_token(def)) {
        all.push(def.text);
        all.extend(def.labels.iter().copied());
    }
    let mut text = all.join("\n");
    for (keyword, token) in KEYWORD_TOKENS {
        if text.to_lowercase().contains(keyword) {
            text.push('\n');
            text.push_str(token);
        }
    }
    text
}

/// Whether `text` makes a token whose name is its subtypes `words` (CR 111.4): "[words] [type words] token", not then
/// "named ...".
fn makes_unnamed(text: &str, words: &str) -> bool {
    let lower = text.to_lowercase();
    let words = words.to_lowercase();
    let mut from = 0;
    while let Some(at) = lower[from..].find(&words) {
        let start = from + at;
        from = start + words.len();
        // The subtypes start a word.
        if lower[..start].chars().next_back().is_some_and(char::is_alphanumeric) {
            continue;
        }
        let mut rest = &lower[from..];
        while let Some(next) = TYPE_WORDS.iter().find_map(|w| rest.strip_prefix(' ').and_then(|r| r.strip_prefix(w))) {
            rest = next;
        }
        if let Some(after) = rest.strip_prefix(" token") {
            let after = after.strip_prefix('s').unwrap_or(after);
            if !after.starts_with(" named ") && !after.starts_with(char::is_alphanumeric) {
                return true;
            }
        }
    }
    false
}

/// Whether `text` names a token `name`: "a ... token named [name]", or "create [name], a ... token".
fn names(text: &str, name: &str) -> bool {
    text.contains(&format!("named {name}")) || text.contains(&format!("{name}, a "))
}

/// Whether `def` is a token's definition: a permanent's characteristics written in code, not a card's, an emblem's (which
/// has no card type) or a copy effect's exception.
fn is_token(def: &CardDef) -> bool {
    !def.oracle && def.dungeon.is_empty() && [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Land, CardType::Planeswalker, CardType::Battle].iter().any(|&t| def.types.has(t)) && !def.name.contains("(copy exception)")
}

#[test]
fn token_names_are_what_the_creating_text_says() {
    let mut wrong = Vec::new();
    let mut checked = 0;
    for (card, extras) in cantrip_cards::files() {
        let text = texts(card, extras);
        let tokens = extras.iter().flat_map(|&def| std::iter::once(def).chain(def.back));
        for def in tokens.filter(|def| is_token(def)) {
            checked += 1;
            if !def.token_words.is_empty() {
                let words: Vec<&str> = def.token_words.iter().map(|s| s.name()).collect();
                let words = words.join(" ");
                if def.name != format!("{words} Token") {
                    wrong.push(format!("{}: its token {} is not named for its subtypes, {words}", card.name, def.name));
                } else if !makes_unnamed(&text, &words) {
                    wrong.push(format!("{}: no \"{words} ... token\" in its text names {} (written with `token!`)", card.name, def.name));
                }
            } else if let Some(&(_, said)) = PREDEFINED_NAMED.iter().find(|(name, _)| *name == def.name) {
                if !text.contains(said) && !text.to_lowercase().contains(&said.to_lowercase()) {
                    wrong.push(format!("{}: makes the predefined {} (CR 111.10) without saying \"{said}\"", card.name, def.name));
                }
            } else if !names(&text, def.name) {
                wrong.push(format!(
                    "{}: its token is written as named {:?}, which its text doesn't name; a token the text doesn't name is `token!(..)` (CR 111.4)",
                    card.name, def.name
                ));
            }
        }
    }
    assert!(checked > 100, "only {checked} token definitions found");
    assert!(wrong.is_empty(), "token names that differ from the text making them (CR 111.4):\n{}", wrong.join("\n"));
}

#[test]
fn a_token_is_named_for_its_subtypes_in_the_texts_order() {
    static HYDRA: CardDef = CardDef { types: TypeSet::of(&[CardType::Creature]), ..token!(Phyrexian Hydra) };
    assert_eq!(HYDRA.name, "Phyrexian Hydra Token");
    assert_eq!(HYDRA.token_words, &[Subtype::Phyrexian, Subtype::Hydra]);
    assert!(HYDRA.subtypes.has(Subtype::Phyrexian) && HYDRA.subtypes.has(Subtype::Hydra));
    assert!(!HYDRA.has_cost);
    // A text change of a creature type the creating text names renames it (CR 612.2a); one of a word it doesn't, doesn't.
    let elf = Words::NONE.changed(Word::Subtype(Subtype::Hydra), Word::Subtype(Subtype::Elf));
    assert_eq!(HYDRA.token_name_under(elf), Some("Phyrexian Elf Token"));
    assert!(std::ptr::eq(HYDRA.token_name_under(elf).unwrap(), HYDRA.token_name_under(elf).unwrap()), "made once");
    let goblin = Words::NONE.changed(Word::Subtype(Subtype::Goblin), Word::Subtype(Subtype::Elf));
    assert_eq!(HYDRA.token_name_under(goblin), None);
    static TWO_WORDS: CardDef = CardDef { types: TypeSet::of(&[CardType::Creature]), ..token!(TimeLord AssemblyWorker) };
    assert_eq!(TWO_WORDS.name, "Time Lord Assembly-Worker Token");
}
