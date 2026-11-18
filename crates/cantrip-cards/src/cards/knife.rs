use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object_during_your_turn, equip};
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::{have_first_strike, plus_pt};
use cantrip_oracle as oracle;

/// A Clue Equipment (CR 205.3g: an artifact type, here on a nontoken Equipment): its own "{2}, Sacrifice this Equipment: Draw a
/// card" is its Clue ability written on the card. "During your turn, equipped creature gets +1/+0 and has first strike" is
/// two static abilities that apply only on its controller's turn (CR 611.3).
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}").and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 1), "{2}, Sacrifice this Equipment: Draw a card."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object_during_your_turn, plus_pt::<1, 0>, "During your turn, equipped creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object_during_your_turn, have_first_strike, "During your turn, equipped creature has first strike."),
    ],
    ..oracle::KNIFE
};
