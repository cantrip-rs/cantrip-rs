use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{B}").and(TAP), |g, item| effects::surveil(g, item, 1), "{B}, {T}: Surveil 1.")],
    ..oracle::WRETCHED_DOLL
};
