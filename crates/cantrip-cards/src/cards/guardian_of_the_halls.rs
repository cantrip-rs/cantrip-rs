use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{5}{G}{G}"), effects::three_plus_one_counters_on_this, "{5}{G}{G}: Put three +1/+1 counters on this creature.")],
    ..oracle::GUARDIAN_OF_THE_HALLS
};
