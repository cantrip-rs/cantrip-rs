use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{U}").and(SACRIFICE_THIS),
        effects::counter_first_target_unless_pays::<3>,
        "{U}, Sacrifice this creature: Counter target instant or sorcery spell unless its controller pays {3}.",
    )
    .targets(&[targets::INSTANT_OR_SORCERY_SPELL])],
    ..oracle::DIVERSION_UNIT
};
