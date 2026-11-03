use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, effects::destroy_first_target, "{T}: Destroy target blue creature.")
        .targets(&[targets::creature_of_color::<{ Color::Blue as u8 }>()])],
    ..oracle::SPINAL_VILLAIN
};
