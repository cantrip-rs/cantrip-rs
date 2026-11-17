use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, plus_pt};
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Clue Equipment. "{2}, Sacrifice this Equipment: Draw a card." (CR 602.2, 701.21): sacrificing it is part of the cost. The ability the
/// equipped creature has (CR 613.1f), "Whenever this creature attacks, surveil 2", is the creature's own: its controller surveils.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}").and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 1), "{2}, Sacrifice this Equipment: Draw a card."), equip(mana("{2}"), "Equip {2}")],
    delayed: &[triggered(THIS_ATTACKS, |g, item| effects::surveil(g, item, 2), "Whenever this creature attacks, surveil 2.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(
            Layer::Ability,
            attached_object,
            grant_delayed_ability,
            "Equipped creature has \"Whenever this creature attacks, surveil 2.\"",
        ),
    ],
    ..oracle::CANDLESTICK
};
