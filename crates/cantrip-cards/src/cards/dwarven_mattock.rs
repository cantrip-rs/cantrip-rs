use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, plus_pt};
use cantrip_mechanics::triggers::{ward, THIS_ENTERS};
use cantrip_mechanics::{lands, targets};
use cantrip_oracle as oracle;

/// "When this Equipment enters, attach it to target Dwarf you control." (CR 701.3a, 301.5): the Dwarf is read through the words of
/// this card's text (CR 612). The Equipment attaches to the target only if it is still legal as the ability resolves and is still
/// on the battlefield (CR 608.2b, 701.3b); a Dwarf that is not a creature cannot be equipped, so nothing is attached to it.
/// "Equipped creature gets +2/+2 and has ward {1}." (CR 702.21a): the ward is the Equipment's delayed ability number 0, granted
/// to the creature it is attached to.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target Dwarf you control.")
            .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Dwarf) }>)]),
        equip(mana("{3}"), "Equip {3}"),
    ],
    delayed: &[ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 2>, "Equipped creature gets +2/+2."),
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Equipped creature has ward {1}."),
    ],
    ..oracle::DWARVEN_MATTOCK
};
