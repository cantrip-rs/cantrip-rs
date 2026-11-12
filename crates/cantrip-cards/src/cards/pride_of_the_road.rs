use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    delayed: &[triggered(BEGINNING_OF_YOUR_COMBAT, effects::target_gains_double_strike, "At the beginning of combat on your turn, target creature or Vehicle you control gains double strike until end of turn.")
        .targets(&[targets::CREATURE_OR_VEHICLE_YOU_CONTROL])],
    statics: &[max_speed("Max speed \u{2014} At the beginning of combat on your turn, target creature or Vehicle you control gains double strike until end of turn.")],
    ..oracle::PRIDE_OF_THE_ROAD
};
