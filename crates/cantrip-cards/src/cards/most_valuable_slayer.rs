use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        effects::target_creature_gets_and_gains_first_strike::<1, 0>,
        "Whenever you attack, target attacking creature gets +1/+0 and gains first strike until end of turn.",
    )
    .targets(&[targets::ATTACKING_CREATURE])],
    ..oracle::MOST_VALUABLE_SLAYER
};
