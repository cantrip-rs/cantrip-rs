use std::sync::OnceLock;

use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

/// "When you discard this card": the discard of this very card, wherever it went (CR 701.9a).
fn this_discarded(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Discarded { card, .. } if *card == source => Fired::PLAIN,
        _ => None,
    }
}

fn gain_three(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

/// A 2/2 with "When you discard this card, you gain 3 life."
static BONES: CardDef = CardDef {
    name: "Probe Bones",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[triggered(when(EventMask::DISCARDED, this_discarded), gain_three, "When you discard this card, you gain 3 life.")
        .functions_in(WHERE_A_DISCARD_ENDS)],
    ..CardDef::BLANK
};

/// "If a card would be discarded, put it on top of its owner's library instead of into their graveyard" (Library of
/// Leng's replacement, without its choice): the card is not revealed.
fn discarded_to_a_graveyard(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.reason == ZoneReason::Discard && z.to.is(ZoneKind::Graveyard))
}

fn library_instead(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::library(q.obj(z.object).owner);
    }
}

static LENG: CardDef = CardDef {
    name: "Probe Leng",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Artifact]),
    replacements: &[replacement(ReplacementKind::ZoneChange, discarded_to_a_graveyard, library_instead, "puts a discarded card on top of its owner's library")],
    ..CardDef::BLANK
};

/// "... reveal it and put it on top of its owner's library instead": the same, revealed as it goes (CR 701.20a).
fn library_instead_revealed(q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    library_instead(q, asker, proposal);
    if let Proposal::ZoneChange(z) = proposal {
        z.reveal = true;
    }
}

static REVEALING_LENG: CardDef = CardDef {
    name: "Probe Revealing Leng",
    replacements: &[replacement(ReplacementKind::ZoneChange, discarded_to_a_graveyard, library_instead_revealed, "reveals a discarded card and puts it on top of its owner's library")],
    ..LENG
};

/// Probe Bones that, as Blightsteel Colossus, is revealed and shuffled into its owner's library instead of going to a
/// graveyard from anywhere.
static SHUFFLING_BONES: CardDef = CardDef {
    name: "Probe Shuffling Bones",
    replacements: &[cantrip_mechanics::replacements::REVEAL_AND_SHUFFLE_INTO_LIBRARY_INSTEAD_OF_GRAVEYARD],
    ..BONES
};

/// Library of Leng's own replacement, with its choice: "If an effect causes you to discard a card, discard it, but you may put
/// it on top of your library instead of into your graveyard." Unrevealed.
fn you_discard_to_a_graveyard(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    discarded_to_a_graveyard(q, asker, proposal) && matches!(proposal, Proposal::ZoneChange(z) if q.obj(z.object).owner == asker.controller)
}

const ON_TOP: Question = Question::new("Library", "put the discarded card on top of your library?", "", "On top", "Graveyard");

static OPTIONAL_LENG: CardDef = CardDef {
    name: "Probe Optional Leng",
    replacements: &[replacement(ReplacementKind::ZoneChange, you_discard_to_a_graveyard, library_instead, "may put a discarded card on top of its owner's library")
        .optional(&ON_TOP)],
    ..LENG
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&BONES, &LENG, &REVEALING_LENG, &OPTIONAL_LENG, &SHUFFLING_BONES])))
}

pub(crate) fn play(name: &str, text: &str) {
    let script = GameScript::parse(text, name).unwrap_or_else(|e| panic!("{name}: {e}"));
    if let Err(failure) = run_on(&script, table()) {
        panic!("{name}:\n{}", failure.report);
    }
}

