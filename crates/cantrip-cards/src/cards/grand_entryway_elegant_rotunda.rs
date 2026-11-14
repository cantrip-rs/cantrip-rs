//! Grand Entryway // Elegant Rotunda, a Room (CR 709.5) whose doors are each a trigger of unlocking it. See Unholy Annex //
//! Ritual Chamber for the Room itself.

use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

pub static GRAND_ENTRYWAY: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_LEFT_DOOR,
        |g, item| {
            tokens::create(g, item.controller, &tokens::GLIMMER, 1)?;
            Ok(())
        },
        "When you unlock this door, create a 1/1 white Glimmer enchantment creature token.",
    )],
    ..oracle::GRAND_ENTRYWAY
};

pub static ELEGANT_ROTUNDA: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_RIGHT_DOOR,
        effects::plus_one_counter_on_each_target,
        "When you unlock this door, put a +1/+1 counter on each of up to two target creatures.",
    )
    .targets(&[targets::CREATURE.between(0, 2)])],
    ..oracle::ELEGANT_ROTUNDA
};

pub static CARD: CardDef = CardDef { split: Some((&GRAND_ENTRYWAY, &ELEGANT_ROTUNDA)), ..oracle::GRAND_ENTRYWAY_ELEGANT_ROTUNDA };

pub static EXTRA: &[&CardDef] = &[&GRAND_ENTRYWAY, &ELEGANT_ROTUNDA, &tokens::GLIMMER];
