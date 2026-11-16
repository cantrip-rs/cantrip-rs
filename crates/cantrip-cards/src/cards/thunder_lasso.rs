use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        // CR 701.3b: it is attached only if the target is still a legal one as this resolves; nothing is otherwise.
        triggered(triggers::THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        // CR 508.5: the defending player is the one the equipped creature attacks; a tapped target is legal and stays tapped (ruling).
        triggered(triggers::ATTACHED_CREATURE_ATTACKS_SUBJECT, effects::tap_targets, "Whenever equipped creature attacks, tap target creature defending player controls.")
            .targets(&[targets::CREATURE_DEFENDING_PLAYER_CONTROLS]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    ..oracle::THUNDER_LASSO
};
