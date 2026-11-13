use cantrip_api::*;
use cantrip_mechanics::{lands, statics, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_treasure, "When this creature enters, create a Treasure token.")],
    statics: &[static_ability(
        Layer::PtModify,
        statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Pirate) }>,
        statics::plus_pt::<1, 1>,
        "Other Pirates you control get +1/+1.",
    )],
    ..oracle::CORSAIR_CAPTAIN
};
