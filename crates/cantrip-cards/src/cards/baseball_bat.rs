use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::{EQUIPPED_CREATURE_ATTACKS, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        // CR 701.3b: it is attached only if the target is still a legal one as this resolves; nothing is otherwise.
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        triggered(EQUIPPED_CREATURE_ATTACKS, effects::tap_targets, "Whenever equipped creature attacks, tap up to one target creature.")
            .targets(&[targets::CREATURE.between(0, 1)]),
        equip(mana("{3}"), "Equip {3}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    ..oracle::BASEBALL_BAT
};
