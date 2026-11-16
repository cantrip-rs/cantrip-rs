use cantrip_api::*;
use cantrip_mechanics::{attach, effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            tap_untapped(1, attach::attached_to_it),
            effects::target_creature_gets::<1, 2>,
            "Tap enchanted land: Target blocking creature gets +1/+2 until end of turn. Activate only if enchanted land is untapped.",
        )
        .targets(&[targets::BLOCKING_CREATURE])
        .condition(attach::enchanted_permanent_untapped),
    ],
    enchant: Some(targets::LAND_YOU_CONTROL),
    ..oracle::EARTHLORE
};
