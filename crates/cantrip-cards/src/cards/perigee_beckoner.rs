use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::{effects, targets, warp};
use cantrip_oracle as oracle;

/// The return trigger is the card's second delayed ability, after warp's exile (`warp::EXILE` is the first).
const RETURN_TRIGGER: u8 = 1;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::target_creature_gets_and_gains_dies_trigger::<2, 0, RETURN_TRIGGER>,
        "When this creature enters, until end of turn, another target creature you control gets +2/+0 and gains \"When this creature dies, return it to the battlefield tapped under its owner's control.\"",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ways: &[warp::way(mana("{1}{B}"), "Warp {1}{B}")],
    end_step_riders: warp::RIDERS,
    delayed: &[
        warp::EXILE,
        triggered(THIS_DIES, effects::return_it_tapped, "When this creature dies, return it to the battlefield tapped under its owner's control."),
    ],
    ..oracle::PERIGEE_BECKONER
};
