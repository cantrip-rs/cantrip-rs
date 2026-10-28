use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::{attacks_each_combat_if_able, itself};
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, |g, item| effects::discard_hand_then_draw(g, item, 3), "Whenever this creature attacks, discard your hand, then draw three cards.")],
    statics: &[static_ability(Layer::Ability, itself, attacks_each_combat_if_able, "This creature attacks each combat if able.")],
    madness: Some(mana("{1}{R}")),
    ..oracle::ANJES_RAVAGER
};
