//! Meat Locker // Drowned Diner, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// Meat Locker: "When you unlock this door, tap up to one target creature and put two stun counters on it."
pub static MEAT_LOCKER: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        effects::tap_and_stun::<2>,
        "When you unlock this door, tap up to one target creature and put two stun counters on it.",
    )
    .targets(&[targets::CREATURE.optional()])],
    ..oracle::MEAT_LOCKER
};

/// Drowned Diner: "When you unlock this door, draw three cards, then discard a card."
pub static DROWNED_DINER: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_RIGHT_DOOR,
        effects::draw_three_then_discard,
        "When you unlock this door, draw three cards, then discard a card.",
    )],
    ..oracle::DROWNED_DINER
};

pub static CARD: CardDef = CardDef { split: Some((&MEAT_LOCKER, &DROWNED_DINER)), ..oracle::MEAT_LOCKER_DROWNED_DINER };

pub static EXTRA: &[&CardDef] = &[&MEAT_LOCKER, &DROWNED_DINER];
