//! Where a triggered ability functions (CR 113.6, `TriggeredDef::functions`) is the battlefield unless its trigger condition
//! can't trigger from there (CR 113.6k) - then the shared trigger says so ("when you cast this spell" is
//! `when_this_spell_is_cast`, functioning from the stack) - or its own text puts it in a zone (CR 113.6b, 113.6m), which
//! the card declares (`AbilityDef::functions_in`). Either way it is held here to the words: each triggered ability's zones
//! are what its `text` says, and the zones a card's abilities function in are what its Oracle text's trigger conditions
//! say. A card that forgets the declaration, or a matcher made with `when` for a condition that can't trigger from the
//! battlefield, fails here rather than never triggering.

use cantrip_api::*;

/// The keyword abilities whose trigger is "when you cast this spell" (CR 702.40a storm, 702.56a replicate, 702.60a ripple,
/// 702.78a conspire, 702.85a cascade, 702.153a casualty), whose ability text may be the keyword alone.
const CAST_KEYWORDS: [&str; 6] = ["storm", "replicate", "ripple", "conspire", "cascade", "casualty"];

/// A printed keyword, with its parameter if any, rather than the beginning of a card name such as Storm Seeker.
fn starts_with_cast_keyword(text: &str) -> bool {
    CAST_KEYWORDS.iter().any(|keyword| {
        let Some(rest) = text.strip_prefix(keyword) else { return false };
        let rest = rest.trim_start();
        rest.is_empty() || rest.starts_with(['.', ','])
            || matches!(*keyword, "replicate" | "ripple" | "casualty")
                && (rest.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '{' | '\u{2014}' | '-'))
                    || rest == "x" || rest.starts_with("x.") || rest.starts_with("x,") || rest.starts_with("x "))
    })
}

/// The zones the words of a triggered ability say it functions in (CR 113.6), for a card named `name`: the union over its
/// trigger conditions (CR 113.6k: "when you cast this spell and whenever this creature attacks" has two, functioning in
/// different zones), each of which functions
///
/// - on the stack, if it is the spell's own cast: "when you cast this spell", a reflexive "when you do" of an additional
///   cost paid to cast it ("... copy this spell"), or a keyword whose trigger is its cast (CR 113.6k);
/// - in every zone a discarded card can wind up in, a library included, if it is the card's own cycling or discarding (CR
///   113.6k, 702.29c), even if what it does then returns it "from your graveyard" (Sand Golem);
/// - in a graveyard, if it is the card's own move into a graveyard other than from the battlefield ("when this card is put
///   into a graveyard from anywhere", "... into your graveyard from your library": CR 113.6k);
/// - in the zone the ability names for its own card, "this card from your graveyard", "if this card is in your graveyard"
///   and the like for a hand and exile (CR 113.6b, 113.6m) - unless the condition itself puts the card into that zone
///   ("when this creature dies, return this card from your graveyard": CR 113.6m's exception, a leaves-the-battlefield
///   ability that functions on the battlefield);
/// - otherwise on the battlefield (CR 113.6).
fn said_zones(text: &str, name: &str) -> ZoneKinds {
    let text = without_reminders(&text.to_ascii_lowercase().replace('\u{2014}', "-"));
    let name = name.to_ascii_lowercase();
    if starts_with_cast_keyword(&text) {
        return ON_THE_STACK;
    }
    let condition = text.split(", ").next().unwrap_or_default();
    let condition = condition.split(" - ").last().unwrap_or_default();
    let mut conditions = vec![condition.to_string()];
    for joint in [" and whenever ", " and when ", " or whenever "] {
        conditions = conditions.iter().flat_map(|c| c.split(joint).map(str::to_string).collect::<Vec<_>>()).collect();
    }
    conditions.iter().fold(ZoneKinds::NONE, |zones, c| zones.union(condition_zones(c, &text, &name)))
}

/// Every zone a discarded card can wind up in, a replacement effect's included (CR 113.6k, 702.29c, 614.1): read here
/// apart from the engine's `cantrip_api::WHERE_A_DISCARD_ENDS`, which the abilities are held to.
const WHERE_A_DISCARD_ENDS: ZoneKinds =
    ZoneKinds::of(&[ZoneKind::Graveyard, ZoneKind::Exile, ZoneKind::Library, ZoneKind::Hand, ZoneKind::Battlefield]);

/// Whether the subject of `condition` before `at` is the ability's own object: "this creature", "this card", or its card's
/// name (or the start of it, "Emrakul").
fn itself(condition: &str, at: usize, name: &str) -> bool {
    let subject = condition[..at].trim_start_matches("whenever ").trim_start_matches("when ").trim();
    subject.starts_with("this ") || (!subject.is_empty() && name.starts_with(subject))
}

/// The zones one trigger condition functions in (`said_zones`); `text` is the whole ability's.
fn condition_zones(condition: &str, text: &str, name: &str) -> ZoneKinds {
    if condition.contains("cast this spell") || (condition == "when you do" && text.contains("this spell")) {
        return ON_THE_STACK;
    }
    if ["you cycle this card", "you discard this card", "you cycle or discard this card", "causes you to discard this card"]
        .iter()
        .any(|w| condition.contains(w))
    {
        // Even when what it does then moves the card out of its graveyard (Sand Golem, Pure Intentions, Mangara's Blessing:
        // "return this card from your graveyard ... at the beginning of the next end step"): the discard is the event, not a
        // move to the graveyard, so it triggers wherever the discard ends (a ruling of the owner's: Megrim triggers when
        // Leyline of the Void exiles the discarded card), and its return finds nothing there.
        return WHERE_A_DISCARD_ENDS;
    }
    // The zones the condition itself puts its own object into (CR 113.6m's exception).
    let mut put_into = ZoneKinds::NONE;
    for (verb, zone) in [
        (" dies", ZoneKind::Graveyard),
        (" is put into a graveyard", ZoneKind::Graveyard),
        (" is put into your graveyard", ZoneKind::Graveyard),
        (" is put into exile", ZoneKind::Exile),
        (" is exiled", ZoneKind::Exile),
        (" is put into your hand", ZoneKind::Hand),
        (" is put into its owner's hand", ZoneKind::Hand),
        (" is returned to", ZoneKind::Hand),
    ] {
        if let Some(at) = condition.find(verb)
            && itself(condition, at, name)
        {
            if zone == ZoneKind::Graveyard && verb != " dies" && !condition[at..].contains("from the battlefield") {
                return IN_A_GRAVEYARD;
            }
            put_into = put_into.with(zone);
        }
    }
    let mut zones = ZoneKinds::NONE;
    for (words, zone) in [("your graveyard", ZoneKind::Graveyard), ("your hand", ZoneKind::Hand), ("exile", ZoneKind::Exile)] {
        if !put_into.contains(zone)
            && (text.contains(&format!("this card from {words}")) || text.contains(&format!("this card is in {words}"))
                || zone == ZoneKind::Exile && text.contains("if this card is exiled"))
        {
            zones = zones.with(zone);
        }
    }
    if zones == ZoneKinds::NONE { ON_THE_BATTLEFIELD } else { zones }
}

/// The reader's own cases, each a printed form.
#[test]
fn the_reader_reads_the_printed_forms() {
    let both = ZoneKinds(ON_THE_STACK.0 | ON_THE_BATTLEFIELD.0);
    for (text, name, zones) in [
        ("At the beginning of your upkeep, if this card is exiled with a scream counter on it, remove a scream counter from it.", "All Hallow's Eve", ZoneKinds::of(&[ZoneKind::Exile])),
        ("When this card is exiled, you gain 1 life.", "Test", ON_THE_BATTLEFIELD),
        ("When you cast this spell, take an extra turn after this one.", "Emrakul, the Aeons Torn", ON_THE_STACK),
        ("When you do, copy this spell for each creature sacrificed this way.", "Plumb the Forbidden", ON_THE_STACK),
        ("Cascade", "Bloodbraid Elf", ON_THE_STACK),
        ("At the beginning of your upkeep, if this card is exiled with a scream counter on it, remove a scream counter from it.", "All Hallow's Eve", ZoneKinds::of(&[ZoneKind::Exile])),
        ("When this creature is exiled, if this card is exiled, you gain 1 life.", "Test", ON_THE_BATTLEFIELD),
        ("When you cast this spell and whenever this creature attacks, destroy up to one target nonland permanent.", "Cityscape Leveler", both),
        ("When Emrakul is put into a graveyard from anywhere, its owner shuffles their graveyard into their library.", "Emrakul, the Aeons Torn", IN_A_GRAVEYARD),
        ("When this card is put into your graveyard from your library, create a 2/2 black Zombie creature token.", "Test", IN_A_GRAVEYARD),
        ("When this creature is put into a graveyard from the battlefield, you gain 2 life.", "Test", ON_THE_BATTLEFIELD),
        ("Whenever another card is put into a graveyard from anywhere, exile it.", "Planar Void", ON_THE_BATTLEFIELD),
        ("Whenever you cast a spell, if it's the second creature spell you cast this turn, you may return this card from your graveyard to the battlefield.", "Vengevine", IN_A_GRAVEYARD),
        ("At the beginning of your upkeep, if this card is in your graveyard, you may pay 1 life. If you do, return it to your hand.", "Master of Death", IN_A_GRAVEYARD),
        // CR 113.6m's exception: the condition puts the card into the graveyard, so it functions where it triggers from.
        ("When this creature dies, return this card from your graveyard to the battlefield.", "Test", ON_THE_BATTLEFIELD),
        ("When Test dies, return this card from your graveyard to your hand.", "Test", ON_THE_BATTLEFIELD),
        ("Whenever another creature dies, return this card from your graveyard to the battlefield.", "Test", IN_A_GRAVEYARD),
        // CR 702.29c: from whatever zone the card winds up in.
        ("When you cycle this card, draw a card.", "Test", WHERE_A_DISCARD_ENDS),
        ("When you cycle or discard this card, you gain 1 life.", "Test", WHERE_A_DISCARD_ENDS),
        ("When you discard this card, you gain 3 life.", "Titanbones, Towering Heart", WHERE_A_DISCARD_ENDS),
        ("When a spell or ability an opponent controls causes you to discard this card, that player loses 5 life.", "Psychic Purge", WHERE_A_DISCARD_ENDS),
        ("When a spell or ability an opponent controls causes you to discard this card, return it to your hand.", "Quagnoth", WHERE_A_DISCARD_ENDS),
        // Not CR 113.6m's graveyard alone: the discard is the event, wherever the card went.
        ("When a spell or ability an opponent controls causes you to discard this card, return this card from your graveyard to the battlefield with a +1/+1 counter on it at the beginning of the next end step.", "Sand Golem", WHERE_A_DISCARD_ENDS),
        ("Whenever you cycle or discard another card, scry 1.", "Test", ON_THE_BATTLEFIELD),
        ("Offspring {2} (You may pay an additional {2} as you cast this spell.)", "Pawpatch Recruit", ON_THE_BATTLEFIELD),
        ("Whenever you cast a colorless spell, exile up to one target permanent that's one or more colors.", "Ugin, Eye of the Storms", ON_THE_BATTLEFIELD),
    ] {
        assert_eq!(names(said_zones(text, name)), names(zones), "{text}");
    }
}

/// The text without its reminder text, which says what a keyword does rather than being an ability (CR 207.2).
fn without_reminders(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Whether an Oracle text has a keyword whose trigger is its cast, or gives one to spells, outside quotes (quoted, it is an
/// ability of what the card makes, an emblem's): "Cascade", "Spells you cast have ripple 4" (Thrumming Stone).
fn cast_keyword(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    text.split(['"', '\u{201c}', '\u{201d}']).step_by(2).flat_map(|part| part.split('\n')).any(|line| {
        let line = without_reminders(line);
        starts_with_cast_keyword(&line) || [" have ", " has "].iter().any(|joint| {
            line.match_indices(joint).any(|(at, _)| starts_with_cast_keyword(&line[at + joint.len()..]))
        })
    })
}

#[test]
fn a_card_name_is_not_a_cast_keyword() {
    for text in ["Storm Seeker deals damage to target player.", "Storm Crow has flying.", "Stormfront Riders has flying."] {
        assert!(!cast_keyword(text), "{text}");
    }
    for text in ["Storm", "Storm (When you cast this spell, copy it.)", "Replicate {U}", "Ripple 4", "Casualty X. The copy isn't legendary.", "Spells you cast have cascade."] {
        assert!(cast_keyword(text), "{text}");
    }
    assert!(!cast_keyword("Create a token with \"Cascade\"."));
}

/// The triggered abilities of an Oracle text: each line, quoted ability and parenthesised reminder cut into sentences, a
/// sentence that starts with "when", "whenever" or "at" (after an ability word's dash) beginning one and the sentences after
/// it in the same part continuing it ("If you do, return this card from your graveyard ..."). A delayed trigger an ability
/// creates ("at the beginning of the next end step") is no ability of its card and functions from no zone (CR 603.7).
fn oracle_abilities(text: &str) -> Vec<String> {
    let text = text.to_ascii_lowercase().replace('\u{2014}', "-");
    let mut abilities = Vec::new();
    for part in text.split(['\n', '(', ')', '"', '\u{201c}', '\u{201d}']) {
        let mut current: Option<String> = None;
        for sentence in part.split(". ") {
            let s = sentence.trim();
            let s = match s.find(" - ") {
                Some(at) if s[..at].split(' ').count() <= 3 => s[at + 3..].trim(),
                _ => s,
            };
            if s.starts_with("when ") || s.starts_with("whenever ") || s.starts_with("at ") {
                abilities.extend(current.take());
                let delayed = s.starts_with("at the beginning of the next") || s.starts_with("at the beginning of your next");
                current = (!delayed).then(|| s.to_string());
            } else if let Some(ability) = current.as_mut() {
                ability.push_str(". ");
                ability.push_str(s);
            }
        }
        abilities.extend(current);
    }
    abilities
}

/// The zones, by name, for a failure's message.
fn names(zones: ZoneKinds) -> Vec<&'static str> {
    [ZoneKind::Battlefield, ZoneKind::Stack, ZoneKind::Hand, ZoneKind::Graveyard, ZoneKind::Exile, ZoneKind::Library, ZoneKind::Command]
        .into_iter()
        .filter(|&z| zones.contains(z))
        .map(ZoneKind::name)
        .collect()
}

fn off_the_battlefield(zones: ZoneKinds) -> ZoneKinds {
    ZoneKinds(zones.0 & !ON_THE_BATTLEFIELD.0)
}

#[test]
fn functioning_zones_are_what_the_words_say() {
    let table = CardTable::new(cantrip_cards::all());
    let mut wrong = Vec::new();
    let (mut stack, mut graveyard) = (0, 0);
    for card in &table.cards {
        let mut declared = ZoneKinds::NONE;
        for a in card.abilities {
            let AbilityDef::Triggered(t) = a else { continue };
            // A state trigger's condition is a state of the permanent (CR 603.8), never a zone's.
            if t.state.is_some() {
                continue;
            }
            let said = said_zones(t.text, card.name);
            if t.functions != said {
                wrong.push(format!("{}: \"{}\" functions in {:?}, its words say {:?}", card.name, t.text, names(t.functions), names(said)));
            }
            declared = declared.union(off_the_battlefield(t.functions));
        }
        // The abilities it gives spells (Thrumming Stone's ripple): one a spell has functions where a spell's does.
        for a in card.delayed {
            if let AbilityDef::Triggered(t) = a
                && t.functions == ON_THE_STACK
            {
                declared = declared.union(ON_THE_STACK);
            }
        }
        if card.text.is_empty() {
            continue;
        }
        let mut oracle = oracle_abilities(card.text).iter().fold(ZoneKinds::NONE, |z, a| z.union(off_the_battlefield(said_zones(a, card.name))));
        if cast_keyword(card.text) {
            oracle = oracle.union(ON_THE_STACK);
        }
        stack += usize::from(oracle.contains(ZoneKind::Stack));
        graveyard += usize::from(oracle.contains(ZoneKind::Graveyard));
        if oracle != declared {
            wrong.push(format!("{}: its Oracle text's triggered abilities function off the battlefield in {:?}, its abilities in {:?}", card.name, names(oracle), names(declared)));
        }
    }
    // The reading finds them at all: Emrakul, the Promised End's cast trigger, Bloodghast's return from the graveyard.
    assert!(stack > 0 && graveyard > 0, "no Oracle text read as functioning on the stack or in a graveyard");
    assert!(wrong.is_empty(), "triggered abilities whose zones differ from their words:\n{}", wrong.join("\n"));
}
