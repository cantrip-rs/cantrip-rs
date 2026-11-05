//! Whether a waiting delayed triggered ability, or a replacement or prevention effect a spell or ability made, refers to its
//! own source - which makes that object a legal "source of your choice" while it waits (CR 609.7a) - is read from its words
//! (`names_its_source`, `CardTable::names_its_source`): a delayed ability's `text`, a granted effect's `text`. Those words are
//! written in the card's file, so this test holds them to the card's Oracle text (`CardDef::text`, generated from its JSON):
//! each is a passage of the Oracle text of its card or of one of its faces (reminder text included), read without its own
//! reminder text and without regard to case, curly quotes, line breaks or a final period - or, for a keyword's own ability
//! that every card with the keyword lists (warp's exile, dash's return), the text that keyword's mechanic gives it, on a card
//! whose Oracle text has the keyword. A card's `delayed` list holds the abilities it grants as well (an Equipment's "ward
//! {1}"), which are read the same.

use cantrip_api::*;
use cantrip_mechanics::*;

/// Oracle words compared as words: lowercase, straight quotes, single spaces, no parentheses.
fn normal(text: &str) -> String {
    let text = text
        .to_lowercase()
        .replace(['\u{2018}', '\u{2019}'], "'")
        .replace(['\u{201c}', '\u{201d}'], "\"")
        .replace(['(', ')'], " ")
        .replace('\u{2212}', "-");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A card and every face it brings.
fn faces(card: &'static CardDef, into: &mut Vec<&'static CardDef>) {
    if into.iter().any(|f| std::ptr::eq(*f, card)) {
        return;
    }
    into.push(card);
    for face in [card.back, card.adventure, card.omen, card.bestow, card.prepare].into_iter().flatten() {
        faces(face, into);
    }
    if let Some((left, right)) = card.split {
        faces(left, into);
        faces(right, into);
    }
}

/// The abilities keyword mechanics give every card with the keyword, whose words are the mechanic's, with the keyword their
/// card's Oracle text names.
fn keyword_abilities() -> Vec<(&'static str, &'static str)> {
    let text = |a: AbilityDef| a.text();
    vec![
        (text(warp::EXILE), "warp"),
        (text(dash::RETURN), "dash"),
        (text(unearth::EXILE), "unearth"),
        (text(mobilize::SACRIFICE), "mobilize"),
        (text(myriad::EXILE), "myriad"),
        (text(paradigm::TRIGGER), "paradigm"),
        (text(earthbend::RETURN), "earthbend"),
        (text(cipher::TRIGGER), "cipher"),
    ]
}

/// `text` without its parenthesized reminder text, which a keyword's mechanic writes for every card that has it (an ability a
/// card grants, "Ward {1} (Whenever this creature ...)"): the derivation never reads it.
fn without_reminder(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

#[test]
fn waiting_texts_are_the_oracle_text() {
    let keywords = keyword_abilities();
    let mut wrong = Vec::new();
    let mut read = 0;
    for card in cantrip_cards::all() {
        let mut group = Vec::new();
        faces(card, &mut group);
        if group.iter().all(|f| f.text.is_empty()) {
            continue;
        }
        let oracle: String = group.iter().map(|f| normal(f.text)).collect::<Vec<_>>().join(" / ");
        for face in &group {
            let delayed = face.delayed.iter().filter_map(|a| match a {
                AbilityDef::Triggered(t) => Some(("delayed ability", t.text)),
                _ => None,
            });
            let replacements = face.granted_replacements.iter().map(|r| ("granted replacement", r.text));
            let preventions = face.granted_preventions.iter().map(|p| ("granted prevention", p.text));
            for (what, text) in delayed.chain(replacements).chain(preventions) {
                read += 1;
                let words = normal(&without_reminder(text));
                let words = words.trim_end_matches('.');
                if !words.is_empty() && oracle.contains(words) {
                    continue;
                }
                if keywords.iter().any(|&(t, keyword)| t == text && oracle.contains(keyword)) {
                    continue;
                }
                wrong.push(format!("{} ({}): {what} {text:?}", card.name, face.name));
            }
        }
    }
    assert!(read > 0, "no waiting text read");
    assert!(wrong.is_empty(), "{} waiting texts that are not their card's Oracle text:\n{}", wrong.len(), wrong.join("\n"));
}

