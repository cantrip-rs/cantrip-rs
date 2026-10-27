//! Cycling (CR 702.29a) and typecycling (CR 702.29e) are cycling abilities (CR 702.29f): discarding a card to pay one's
//! cost is cycling it, which the core announces as `Event::Cycled` ("when you cycle this card", CR 702.29c), and paying
//! any other "Discard this card" cost (channel) is not. Which abilities those are is `ActivatedDef::cycling`, set by the
//! keyword's shared mechanics (`triggers::cycling`, `typecycling`); a card that wrote one by hand would forget it, and
//! nothing would trigger on its cycles. So the flag is held here to the Oracle text, both ways: each line that is a
//! cycling keyword ("Cycling {2}", "Plainscycling {1}", "Basic landcycling {2}") is an ability with the flag, and each
//! ability with the flag is such a line.

use cantrip_api::*;

/// Whether `line` is a cycling or typecycling keyword ability: its keyword, the word or two before the cost, ends in
/// "cycling".
fn is_cycling_line(line: &str) -> bool {
    let words = line.split(['{', '\u{2014}', '(']).next().unwrap_or_default().trim().to_lowercase();
    words.ends_with("cycling") && words.split(' ').count() <= 2
}

/// The keywords of one line of Oracle text, its reminder text left out: "Swampcycling {2}, mountaincycling {2}" (Igneous
/// Pouncer) is two typecycling abilities on one line, each its own (CR 702.29e). A comma inside a cost ("Cycling\u{2014}Pay
/// 2 life, ...") is never one between keywords, since a cost in braces holds no comma and a cost after a dash ends the line.
fn keyword_parts(line: &str) -> Vec<String> {
    let bare = match line.find(" (") {
        Some(at) if line.ends_with(')') => &line[..at],
        _ => line,
    };
    if bare.contains('\u{2014}') { vec![bare.to_string()] } else { bare.split(", ").map(str::to_string).collect() }
}

#[test]
fn cycling_abilities_are_declared_exactly_where_the_oracle_text_says_them() {
    let table = CardTable::new(cantrip_cards::all());
    let mut wrong = Vec::new();
    let mut said = 0;
    for card in table.cards.iter() {
        let lines = card.text.lines().flat_map(keyword_parts).filter(|p| is_cycling_line(p)).count();
        said += lines;
        let mut flagged = 0;
        for ability in card.abilities {
            let AbilityDef::Activated(a) = ability else { continue };
            let says = is_cycling_line(a.text);
            flagged += usize::from(a.cycling);
            if a.cycling != says {
                wrong.push(format!("{}: the ability \"{}\" has cycling {}, its text {}", card.name, a.text, a.cycling, if says { "says it" } else { "doesn't" }));
            }
            if a.cycling && !(a.cost.discard_self && a.from == ZoneKind::Hand) {
                wrong.push(format!("{}: the cycling ability \"{}\" isn't \"[cost], Discard this card\" from the hand (CR 702.29a)", card.name, a.text));
            }
        }
        if flagged != lines {
            wrong.push(format!("{}: {lines} cycling lines in its Oracle text, {flagged} cycling abilities in its file", card.name));
        }
    }
    assert!(said > 0, "no Oracle text read as a cycling keyword");
    assert!(wrong.is_empty(), "cycling declared otherwise than the Oracle text says:\n{}", wrong.join("\n"));
}

#[test]
fn each_cycling_line_is_read_as_one() {
    for (line, cycling) in [
        ("Cycling {2} ({2}, Discard this card: Draw a card.)", true),
        ("Cycling {X}{1}{U} ({X}{1}{U}, Discard this card: Draw a card.)", true),
        ("Plainscycling {1} ({1}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)", true),
        ("Basic landcycling {2} ({2}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)", true),
        ("Cycling\u{2014}Pay 2 life.", true),
        ("Channel \u{2014} {1}{G}, Discard Boseiju, Who Endures: Destroy target artifact, enchantment, or nonbasic land an opponent controls.", false),
        ("When you cycle this card, create an X/X blue Shark creature token with flying.", false),
        ("Whenever you cycle or discard another card, scry 1.", false),
    ] {
        assert_eq!(is_cycling_line(line), cycling, "{line}");
    }
}

#[test]
fn two_typecyclings_on_one_line_are_two_cycling_keywords() {
    let parts = keyword_parts("Swampcycling {2}, mountaincycling {2} ({2}, Discard this card: Search your library for a Swamp or Mountain card, reveal it, put it into your hand, then shuffle.)");
    assert_eq!(parts, ["Swampcycling {2}", "mountaincycling {2}"]);
    assert!(parts.iter().all(|p| is_cycling_line(p)));
    assert_eq!(keyword_parts("Cycling {2} ({2}, Discard this card: Draw a card.)").len(), 1);
}
