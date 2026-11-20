use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_ATTACK;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        effects::target_creature_gets::<1, 0>,
        "Whenever you attack, target attacking Goblin you control gets +1/+0 until end of turn.",
    )
    .targets(&[target(targets::attacking_creature_of_subtype_you_control::<{ lands::sub(Subtype::Goblin) }>)])],
    ..oracle::BOGGART_PRANKSTER
};
