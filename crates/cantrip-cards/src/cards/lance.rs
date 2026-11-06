use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        attach::attached_object,
        statics::have_first_strike,
        "Enchanted creature has first strike.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::LANCE
};
