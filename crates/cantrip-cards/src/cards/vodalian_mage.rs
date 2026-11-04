use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{U}").and(TAP),
        effects::counter_first_target_unless_pays::<1>,
        "{U}, {T}: Counter target spell unless its controller pays {1}.",
    ).targets(&[targets::SPELL])],
    ..oracle::VODALIAN_MAGE
};
