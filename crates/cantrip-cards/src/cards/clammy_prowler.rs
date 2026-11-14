use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::target_creature_cant_be_blocked_this_turn,
        "Whenever this creature attacks, another target attacking creature can't be blocked this turn.",
    )
    .targets(&[targets::ANOTHER_ATTACKING_CREATURE])],
    ..oracle::CLAMMY_PROWLER
};
