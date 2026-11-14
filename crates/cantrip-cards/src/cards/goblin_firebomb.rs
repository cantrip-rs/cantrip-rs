use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{7}, {T}").and(SACRIFICE_THIS),
        effects::destroy_first_target,
        "{7}, {T}, Sacrifice this artifact: Destroy target permanent.",
    ).targets(&[targets::PERMANENT])],
    ..oracle::GOBLIN_FIREBOMB
};
