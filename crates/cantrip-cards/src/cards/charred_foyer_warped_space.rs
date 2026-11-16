//! Charred Foyer // Warped Space, a Room (CR 709.5): each door is only what its text does; the Room has the abilities of
//! its unlocked doors (`layers::doors`, `CardTable::texts`).

use cantrip_api::*;
use cantrip_mechanics::{library, triggers, zone_cast_from};
use cantrip_oracle as oracle;

pub static CHARRED_FOYER: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        library::exile_top_card_play_this_turn,
        "At the beginning of your upkeep, exile the top card of your library. You may play it this turn.",
    )],
    ..oracle::CHARRED_FOYER
};

/// "Once each turn, you may pay {0} rather than pay the mana cost for a spell you cast from exile" (CR 118.9, 601.2b): an
/// alternative cost of {0} the Room gives its controller while this door is unlocked, for a spell cast from any exile zone
/// (the card Charred Foyer exiled, an adventurer's card, a foretold card ...). It is used up by a cast paid for by it this
/// turn (`Event::PaidAlternative` naming the Room): a card cast through Charred Foyer's own permission and paid for in
/// full does not use it, so the once is this alternative's alone. Another Room, or this one come back, is another (CR 400.7).
fn once_each_turn_cast_from_exile(q: &dyn Query, asker: &Asker, spell: ObjectId, player: PlayerId) -> bool {
    player == asker.controller
        && zone_cast_from(q, spell) == Some(ZoneKind::Exile)
        && !q.history().this_turn().iter().any(|r| matches!(r.event, Event::PaidAlternative { by, .. } if by.object == asker.source))
}

pub static WARPED_SPACE: CardDef = CardDef {
    spell_alternatives: &[SpellAlternative {
        text: "Once each turn, you may pay {0} rather than pay the mana cost for a spell you cast from exile.",
        applies: once_each_turn_cast_from_exile,
        cost: InsteadOfManaCost::Mana(ManaCost::ZERO),
        flash: false,
        kind: WayKind::Other,
    }],
    ..oracle::WARPED_SPACE
};

pub static CARD: CardDef = CardDef { split: Some((&CHARRED_FOYER, &WARPED_SPACE)), ..oracle::CHARRED_FOYER_WARPED_SPACE };

pub static EXTRA: &[&CardDef] = &[&CHARRED_FOYER, &WARPED_SPACE];
