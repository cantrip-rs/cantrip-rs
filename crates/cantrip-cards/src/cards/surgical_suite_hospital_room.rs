//! Surgical Suite // Hospital Room, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::triggers::{self, YOU_ATTACK};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Surgical Suite: "When you unlock this door, return target creature card with mana value 3 or less from your graveyard to the
/// battlefield."
pub static SURGICAL_SUITE: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        effects::return_first_target_to_battlefield,
        "When you unlock this door, return target creature card with mana value 3 or less from your graveyard to the battlefield.",
    )
    .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::SURGICAL_SUITE
};

/// Hospital Room: "Whenever you attack, put a +1/+1 counter on target attacking creature." Any attacking creature, not only one
/// of yours.
pub static HOSPITAL_ROOM: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        effects::plus_one_counter_on_target,
        "Whenever you attack, put a +1/+1 counter on target attacking creature.",
    )
    .targets(&[targets::ATTACKING_CREATURE])],
    ..oracle::HOSPITAL_ROOM
};

pub static CARD: CardDef = CardDef { split: Some((&SURGICAL_SUITE, &HOSPITAL_ROOM)), ..oracle::SURGICAL_SUITE_HOSPITAL_ROOM };

pub static EXTRA: &[&CardDef] = &[&SURGICAL_SUITE, &HOSPITAL_ROOM];
