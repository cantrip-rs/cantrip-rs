use cantrip_api::*;
use cantrip_mechanics::{lands, statics, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        statics::itself,
        lands::plus_per_subtype_you_control::<{ lands::sub(Subtype::Treasure) }, false, 1, 1>,
        "Grand Nagus Zek gets +1/+1 for each Treasure you control.",
    )],
    abilities: &[triggered(
        triggers::ANOTHER_CREATURE_DIES,
        tokens::create_a_treasure,
        "Whenever another creature dies, create a Treasure token. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::GRAND_NAGUS_ZEK
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
