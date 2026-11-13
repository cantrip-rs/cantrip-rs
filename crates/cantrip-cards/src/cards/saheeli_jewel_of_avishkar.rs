use cantrip_api::*;
use cantrip_mechanics::statics::have_haste;
use cantrip_mechanics::{lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::NONCREATURE_SPELL_YOU_CAST,
        tokens::create_a_thopter,
        "Whenever you cast a noncreature spell, create a 1/1 colorless Thopter artifact creature token with flying.",
    )],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_of_subtype::<{ lands::sub(Subtype::Thopter) }>,
        have_haste,
        "Thopters you control have haste.",
    )],
    ..oracle::SAHEELI_JEWEL_OF_AVISHKAR
};

pub static EXTRA: &[&CardDef] = &[&tokens::THOPTER];
