use cantrip_api::*;
use cantrip_mechanics::triggers::{another_of_subtype_you_control_enters, THIS_LEAVES};
use cantrip_mechanics::{behold, effects, lands};
use cantrip_oracle as oracle;

/// "Whenever another Elemental you control enters, it deals damage equal to its power to each opponent" (CR 603.6a, 120.3): the Elemental
/// that entered is the source (`StackItem::subject`). "As an additional cost to cast this spell, behold an Elemental and exile it"
/// (CR 601.2b, 701.4a), exiled with the creature the spell becomes (CR 607.2q), which returns it when it leaves the battlefield. A
/// spell that is countered leaves it exiled.
pub static CARD: CardDef = CardDef {
    additional_cost: Some(behold::cost_and_exile(behold::elemental, "Elemental")),
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Elemental) }>),
            effects::subject_deals_damage_equal_to_its_power_to_each_opponent,
            "Whenever another Elemental you control enters, it deals damage equal to its power to each opponent.",
        ),
        triggered(THIS_LEAVES, effects::return_exiled_to_hand, "When this creature leaves the battlefield, return the exiled card to its owner's hand."),
    ],
    ..oracle::CHAMPION_OF_THE_PATH
};
