use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            attach::attached_object,
            cantrip_mechanics::statics::plus_pt::<3, 0>,
            "Enchanted creature gets +3/+0 and can only attack alone.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    // "Can only attack alone" (CR 506.5, 508.1c): a declaration with the enchanted creature has no other attacker.
    combat_groups: &[CombatGroupRule { role: CombatRole::Attacker, about: Some(attach::attached_to_it), at_most: 1 }],
    ..oracle::ERRANTRY
};
