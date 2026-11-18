use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip, equip_quality};
use cantrip_mechanics::statics::{grant_delayed_ability, plus_pt};
use cantrip_mechanics::targets::creature_of_subtype_you_control;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

/// Equipment. "Equipped creature gets +1/+1 and has 'Whenever this creature attacks, draw a card, then discard a card.'" The ability
/// the creature has (CR 613.1f) is its own: its controller draws, then discards. "Equip Pirate {1}" (CR 702.6a) targets a Pirate
/// you control, "Pirate" read through the words of this card's text (CR 612); "Equip {2}" targets any creature you control.
pub static CARD: CardDef = CardDef {
    abilities: &[
        equip_quality(mana("{1}"), &[target(creature_of_subtype_you_control::<{ lands::sub(Subtype::Pirate) }>)], "Equip Pirate {1}"),
        equip(mana("{2}"), "Equip {2}"),
    ],
    delayed: &[triggered(THIS_ATTACKS, effects::loot, "Whenever this creature attacks, draw a card, then discard a card.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(
            Layer::Ability,
            attached_object,
            grant_delayed_ability,
            "Equipped creature has \"Whenever this creature attacks, draw a card, then discard a card.\"",
        ),
    ],
    ..oracle::PIRATE_HAT
};
