use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{4}{U}"), effects::target_creature_cant_be_blocked_this_turn, "{4}{U}: Target creature can't be blocked this turn.")
        .targets(&[targets::CREATURE])],
    ..oracle::ELVENKINGS_HARPER
};
