use cantrip_api::*;
use cantrip_mechanics::statics::{have_flying, plus_pt};
use cantrip_mechanics::{attach, effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Flash is a core keyword.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this Aura enters, draw a card.")],
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, have_flying, "Enchanted creature gets +1/+0 and has flying."),
        static_ability(Layer::PtModify, attach::attached_object, plus_pt::<1, 0>, "Enchanted creature gets +1/+0 and has flying.").continues(),
    ],
    ..oracle::FEATHER_OF_FLIGHT
};
