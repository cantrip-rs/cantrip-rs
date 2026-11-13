use cantrip_api::*;
use cantrip_mechanics::statics::{other_of_subtype_you_control, plus_pt};
use cantrip_mechanics::{lands, tokens, triggers};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::CAT];

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        other_of_subtype_you_control::<{ lands::sub(Subtype::Cat) }>,
        plus_pt::<1, 1>,
        "Other Cats you control get +1/+1.",
    )],
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::this_or_another_nontoken_of_subtype_you_control_enters::<{ lands::sub(Subtype::Cat) }>),
        tokens::create_a_cat,
        "Whenever Arahbo or another nontoken Cat you control enters, create a 1/1 white Cat creature token.",
    )],
    ..oracle::ARAHBO_THE_FIRST_FANG
};
