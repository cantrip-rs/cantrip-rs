use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{U}"), effects::tap_targets, "{U}: Tap target green creature.")
        .targets(&[targets::creature_of_color::<{ Color::Green as u8 }>()])],
    ..oracle::HOMARID_SHAMAN
};
