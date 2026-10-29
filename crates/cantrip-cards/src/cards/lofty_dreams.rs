use cantrip_api::*;
use cantrip_mechanics::statics::{have_flying, plus_pt};
use cantrip_mechanics::{attach::attached_object, effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

// Convoke is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    // "When this Aura enters, draw a card." (CR 603.6a)
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this Aura enters, draw a card.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<2, 2>, "Enchanted creature gets +2/+2."),
        static_ability(Layer::Ability, attached_object, have_flying, "Enchanted creature has flying."),
    ],
    ..oracle::LOFTY_DREAMS
};
