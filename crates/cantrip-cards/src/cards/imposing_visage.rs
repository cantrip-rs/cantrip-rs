use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        attach::attached_object,
        statics::have_menace,
        "Enchanted creature has menace.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::IMPOSING_VISAGE
};
