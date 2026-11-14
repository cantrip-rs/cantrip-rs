//! Glassworks // Shattered Yard, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the
//! abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::triggers::{self, YOUR_END_STEP};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Glassworks: "When you unlock this door, this Room deals 4 damage to target creature an opponent controls."
pub static GLASSWORKS: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        effects::damage_first_target::<4>,
        "When you unlock this door, this Room deals 4 damage to target creature an opponent controls.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::GLASSWORKS
};

/// Shattered Yard: "At the beginning of your end step, this Room deals 1 damage to each opponent."
pub static SHATTERED_YARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        effects::damage_each_opponent_n::<1>,
        "At the beginning of your end step, this Room deals 1 damage to each opponent.",
    )],
    ..oracle::SHATTERED_YARD
};

pub static CARD: CardDef = CardDef { split: Some((&GLASSWORKS, &SHATTERED_YARD)), ..oracle::GLASSWORKS_SHATTERED_YARD };

pub static EXTRA: &[&CardDef] = &[&GLASSWORKS, &SHATTERED_YARD];
