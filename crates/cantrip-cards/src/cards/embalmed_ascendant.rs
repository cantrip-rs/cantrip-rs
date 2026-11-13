use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{CREATURE_YOU_CONTROL_DIES, THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_zombie, "When this creature enters, create a 2/2 black Zombie creature token.")],
    delayed: &[triggered(
        CREATURE_YOU_CONTROL_DIES,
        effects::each_opponent_loses_1_life_you_gain_1,
        "Max speed \u{2014} Whenever a creature you control dies, each opponent loses 1 life and you gain 1 life.",
    )],
    statics: &[max_speed("Max speed \u{2014} Whenever a creature you control dies, each opponent loses 1 life and you gain 1 life.")],
    ..oracle::EMBALMED_ASCENDANT
};

/// The Zombie token the first ability makes.
pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE];
