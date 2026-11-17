use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, have_vigilance, plus_pt};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{2}").and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 1), "{2}, Sacrifice this Equipment: Draw a card."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    // "{3}, {T}: Tap target creature." as the equipped creature has it (CR 113.7, 702.6).
    delayed: &[ability(cost("{3}").and(TAP), effects::tap_targets, "{3}, {T}: Tap target creature.").targets(&[targets::CREATURE])],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, have_vigilance, "Equipped creature has vigilance."),
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Equipped creature has \"{3}, {T}: Tap target creature.\""),
    ],
    ..oracle::WRENCH
};
