//! Whether a triggered ability looks back in time (CR 603.10) is read partly from its own words (`read_zone_words`): "from
//! anywhere" (CR 603.6c) and a card leaving a graveyard (CR 603.10a), which its trigger event can't show. Those words are
//! the ability's `text`, written in its card's file, so this test holds them to the card's Oracle text (`CardDef::text`,
//! generated from its JSON): each of the two is said by one of a card's triggered abilities exactly when the card's Oracle
//! text has a trigger condition that says it - its own lines, the abilities it grants in quotes, and its reminder text.

use cantrip_api::*;

/// The trigger conditions of an Oracle text: each line, quoted ability and parenthesised reminder, cut into sentences, that
/// starts with "when", "whenever" or "at" (after an ability word's dash).
fn oracle_conditions(text: &str) -> Vec<String> {
    let text = text.to_ascii_lowercase().replace('\u{2014}', "-");
    text.split(['\n', '(', ')', '"', '\u{201c}', '\u{201d}'])
        .flat_map(|part| part.split(". "))
        .map(|s| {
            let s = s.trim();
            match s.find(" - ") {
                Some(at) if s[..at].split(' ').count() <= 3 => s[at + 3..].trim().to_string(),
                _ => s.to_string(),
            }
        })
        .filter(|s| s.starts_with("when ") || s.starts_with("whenever ") || s.starts_with("at "))
        .collect()
}

#[test]
fn zone_words_are_read_from_the_oracle_text() {
    let table = CardTable::new(cantrip_cards::all());
    let mut wrong = Vec::new();
    let (mut from_anywhere, mut leaves_graveyard) = (0, 0);
    for card in table.cards.iter().filter(|c| !c.text.is_empty()) {
        let oracle = oracle_conditions(card.text).iter().map(|c| read_zone_words(c)).fold(ZoneWords::default(), |a, w| ZoneWords {
            from_anywhere: a.from_anywhere || w.from_anywhere,
            leaves_graveyard: a.leaves_graveyard || w.leaves_graveyard,
        });
        // A Room's (or other split card's) Oracle text is both halves' text, and each half's abilities are its own (CR 709.5).
        let defs: Vec<&CardDef> = std::iter::once(*card).chain(card.split.into_iter().flat_map(|(left, right)| [left, right])).collect();
        let abilities = defs
            .iter()
            .flat_map(|def| def.abilities.iter().chain(def.delayed))
            .filter_map(|a| match a {
                AbilityDef::Triggered(t) => Some(read_zone_words(t.text)),
                _ => None,
            })
            .fold(ZoneWords::default(), |a, w| ZoneWords {
                from_anywhere: a.from_anywhere || w.from_anywhere,
                leaves_graveyard: a.leaves_graveyard || w.leaves_graveyard,
            });
        from_anywhere += usize::from(oracle.from_anywhere);
        leaves_graveyard += usize::from(oracle.leaves_graveyard);
        if oracle != abilities {
            wrong.push(format!("{}: Oracle {oracle:?}, abilities {abilities:?}", card.name));
        }
    }
    // The reading finds them at all: Planar Void says "from anywhere", Hardened Academic a card leaving a graveyard.
    assert!(from_anywhere > 0 && leaves_graveyard > 0, "no Oracle text read as saying either");
    assert!(wrong.is_empty(), "triggered abilities whose words differ from their card's Oracle text:\n{}", wrong.join("\n"));
}
