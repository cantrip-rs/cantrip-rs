use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{R}"),
            effects::attached_creature_gets::<1, 0>,
            "{R}: Enchanted creature gets +1/+0 until end of turn.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::FIREBREATHING
};
