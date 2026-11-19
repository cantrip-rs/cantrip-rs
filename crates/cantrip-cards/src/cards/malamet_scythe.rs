use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        // CR 701.3b: it is attached only if the target is still a legal one as this resolves; nothing is otherwise.
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{4}"), "Equip {4}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 2>, "Equipped creature gets +2/+2.")],
    ..oracle::MALAMET_SCYTHE
};
