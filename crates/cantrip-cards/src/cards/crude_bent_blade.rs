use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{statics, target_player_sacrifices_a_creature, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, target_player_sacrifices_a_creature, "When this Equipment enters, target opponent sacrifices a creature of their choice.")
            .targets(&[targets::OPPONENT]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, statics::plus_pt::<2, 1>, "Equipped creature gets +2/+1.")],
    ..oracle::CRUDE_BENT_BLADE
};
