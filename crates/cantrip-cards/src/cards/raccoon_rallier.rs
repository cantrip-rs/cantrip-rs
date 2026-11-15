use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{T}"),
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Haste]).0 }>,
        "{T}: Target creature you control gains haste until end of turn. Activate only as a sorcery.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])
    .sorcery_speed()],
    ..oracle::RACCOON_RALLIER
};
