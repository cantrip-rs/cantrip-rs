use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object_during_your_turn, equip};
use cantrip_mechanics::statics::{have_first_strike, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    statics: &[
        static_ability(Layer::PtModify, attached_object_during_your_turn, plus_pt::<2, 0>, "During your turn, equipped creature gets +2/+0."),
        static_ability(Layer::Ability, attached_object_during_your_turn, have_first_strike, "During your turn, equipped creature has first strike."),
    ],
    ..oracle::QUICK_DRAW_KATANA
};
