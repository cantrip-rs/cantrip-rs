use cantrip_api::*;
use cantrip_mechanics::{affinity, effects};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_artifacts],
    abilities: &[ability(cost("{5}{U}"), effects::this_cant_be_blocked_this_turn, "{5}{U}: This creature can't be blocked this turn.")],
    ..oracle::GEARSEEKER_SERPENT
};
