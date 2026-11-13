use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}, {T}").and(SACRIFICE_THIS),
        effects::destroy_first_target,
        "{2}, {T}, Sacrifice this creature: Destroy another target artifact. Activate only as a sorcery.",
    )
    .targets(&[targets::OTHER_ARTIFACT])
    .sorcery_speed()],
    ..oracle::AETHERJACKET
};
