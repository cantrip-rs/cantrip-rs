use cantrip_api::*;
use cantrip_mechanics::{lands, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::itself_while_you_control_a::<{ lands::sub(Subtype::Dragon) }>,
        statics::have_flying,
        "As long as you control a Dragon, this creature has flying.",
    )],
    ..oracle::KARGAN_DRAGONRIDER
};
