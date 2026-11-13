use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::LANDFALL};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        LANDFALL,
        effects::tap_and_stun::<1>,
        "Landfall \u{2014} Whenever a land you control enters, tap target creature an opponent controls and put a stun counter on it. (If a permanent with a stun counter would become untapped, remove one from it instead.)",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::GRAPPLING_KRAKEN
};
