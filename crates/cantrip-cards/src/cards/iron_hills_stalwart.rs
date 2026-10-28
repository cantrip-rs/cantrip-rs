use cantrip_api::*;
use cantrip_mechanics::attach::attach_first_target_to_second;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, attach_first_target_to_second, "When this creature enters, attach target Equipment you control to up to one target creature you control.")
        .targets(&[
            targets::EQUIPMENT_YOU_CONTROL.labelled("Equipment to attach"),
            up_to_one(targets::CREATURE_YOU_CONTROL.filter).labelled("Creature to attach it to"),
        ])],
    ..oracle::IRON_HILLS_STALWART
};
