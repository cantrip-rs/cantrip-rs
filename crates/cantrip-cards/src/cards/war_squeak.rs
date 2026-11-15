use cantrip_api::*;
use cantrip_mechanics::statics::{have_haste, plus_pt};
use cantrip_mechanics::{attach, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Enchant creature. When this Aura enters, target creature an opponent controls can't block this turn. Enchanted creature gets +1/+1
/// and has haste." (CR 303.4, 509.1b, 611.2a, 613.4c, 702.10): any creature may be enchanted, the controller's or an opponent's, and the
/// enters trigger's target is chosen separately (it may be the enchanted creature itself when an opponent controls it).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::first_target_cant_block_this_turn,
            "When this Aura enters, target creature an opponent controls can't block this turn.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
    ],
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, have_haste, "Enchanted creature gets +1/+1 and has haste."),
        static_ability(Layer::PtModify, attach::attached_object, plus_pt::<1, 1>, "Enchanted creature gets +1/+1 and has haste.").continues(),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::WAR_SQUEAK
};
