use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_dies::<{ lands::sub(Subtype::Goblin) }>),
        effects::damage_each_opponent_n::<1>,
        "Whenever another Goblin you control dies, this creature deals 1 damage to each opponent.",
    )],
    ..oracle::BOGGART_CURSECRAFTER
};
