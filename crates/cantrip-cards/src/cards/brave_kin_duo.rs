use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(TAP),
        effects::target_creature_gets::<1, 1>,
        "{1}, {T}: Target creature gets +1/+1 until end of turn. Activate only as a sorcery.",
    )
    .targets(&[targets::CREATURE])
    .sorcery_speed()],
    ..oracle::BRAVE_KIN_DUO
};
