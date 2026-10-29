use cantrip_api::*;
use cantrip_mechanics::statics::{flash_if_you_control_a, other_creatures_you_control, plus_pt};
use cantrip_mechanics::{lands, recruit, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    flash_if: Some(flash_if_you_control_a::<{ lands::sub(Subtype::Human) }>),
    statics: &[static_ability(Layer::PtModify, other_creatures_you_control, plus_pt::<1, 1>, "Other creatures you control get +1/+1.")],
    abilities: &[triggered(triggers::THIS_ENTERS_OR_ATTACKS, recruit::effect, "Whenever this creature enters or attacks, recruit.")],
    ..oracle::BARDS_COMPANY
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
