use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, have_flying, plus_pt};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, you gain 1 life.", the ability the Equipment grants the equipped creature.
pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    delayed: &[cantrip_mechanics::effects::attacks_gain_life::<1>("Whenever this creature attacks, you gain 1 life.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Equipped creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object, have_flying, "Equipped creature has flying."),
        static_ability(
            Layer::Ability,
            attached_object,
            grant_delayed_ability,
            "Equipped creature has \"Whenever this creature attacks, you gain 1 life.\"",
        ),
    ],
    ..oracle::MEDICS_KITESAIL
};
