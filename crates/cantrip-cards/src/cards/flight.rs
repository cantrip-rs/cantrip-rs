use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, statics::have_flying, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    statics: &[static_ability(
        Layer::Ability,
        attached_object,
        have_flying,
        "Enchanted creature has flying.",
    )],
    ..oracle::FLIGHT
};
