use cantrip_api::*;
use cantrip_mechanics::{sneak, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::attacking_creatures_you_control,
        statics::have_double_strike,
        "Attacking creatures you control have double strike.",
    )],
    ways: &[sneak::way(mana("{1}{R}{R}"), "Sneak {1}{R}{R}")],
    ..oracle::RAPHAEL_THE_NIGHTWATCHER
};
