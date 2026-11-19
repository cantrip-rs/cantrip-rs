use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        effects::target_creature_gets_and_gains_vigilance::<1, 1>,
        "{T}: Target creature you control gets +1/+1 and gains vigilance until end of turn. Activate only as a sorcery.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])
    .sorcery_speed()],
    ..oracle::SURLY_FARRIER
};
