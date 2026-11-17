use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, have_vigilance, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{3}"), "Equip {3}")],
    delayed: &[mana_ability(TAP.and(SACRIFICE_THIS), add_any_color(3), "{T}, Sacrifice this creature: Add three mana of any one color.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<3, 3>, "Equipped creature gets +3/+3."),
        static_ability(Layer::Ability, attached_object, have_vigilance, "Equipped creature has vigilance."),
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Equipped creature has \"{T}, Sacrifice this creature: Add three mana of any one color.\""),
    ],
    ..oracle::LOTUS_RING
};
