use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                SACRIFICE_THIS.and(TAP),
                effects::destroy_first_target,
                "{T}, Sacrifice this creature: Destroy target Wall.",
            )
            .targets(&[targets::WALL]),
    ],
    ..oracle::GOBLIN_DIGGING_TEAM
};
