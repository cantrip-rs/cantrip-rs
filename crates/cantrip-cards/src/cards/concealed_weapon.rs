use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{disguise, targets, triggers};
use cantrip_oracle as oracle;

/// "Equipped creature gets +3/+0." "Disguise {2}{R}" (CR 702.168). "When this Equipment is turned face up, attach it to target
/// creature you control." (CR 701.3, 708.8): face down it is a 2/2 creature, face up it is an Equipment again and no creature,
/// so only another creature of yours is a legal target; with none the ability is not put on the stack (CR 603.3d). "Equip {1}{R}".
pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{2}{R}")), "Disguise {2}{R}")],
    abilities: &[
        triggered(triggers::THIS_TURNED_FACE_UP, attach_to_target, "When this Equipment is turned face up, attach it to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{1}{R}"), "Equip {1}{R}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<3, 0>, "Equipped creature gets +3/+0.")],
    ..oracle::CONCEALED_WEAPON
};
