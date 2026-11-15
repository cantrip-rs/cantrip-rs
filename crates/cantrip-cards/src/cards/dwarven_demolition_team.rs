use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, effects::destroy_first_target, "{T}: Destroy target Wall.")
            .targets(&[targets::WALL]),
    ],
    ..oracle::DWARVEN_DEMOLITION_TEAM
};
