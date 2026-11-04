use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{W}").and(TAP),
        effects::destroy_first_target,
        "{1}{W}, {T}: Destroy target black creature.",
    ).targets(&[targets::creature_of_color::<{ Color::Black as u8 }>()])],
    ..oracle::EXORCIST
};
