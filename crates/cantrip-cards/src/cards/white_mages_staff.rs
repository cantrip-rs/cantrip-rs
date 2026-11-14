use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::JOB_SELECT;
use cantrip_mechanics::tokens::HERO_1_1_COLORLESS;
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{also_a, grant_delayed_ability, plus_pt};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[JOB_SELECT, equip(mana("{3}"), "Equip {3}")],
    // "Whenever this creature attacks, you gain 1 life.", the ability the Equipment grants the equipped creature (CR 613.1f).
    delayed: &[effects::attacks_gain_life::<1>("Whenever this creature attacks, you gain 1 life.")],
    statics: &[
        static_ability(Layer::Type, attached_object, also_a::<{ lands::sub(Subtype::Cleric) }>, "Equipped creature is a Cleric in addition to its other types."),
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(
            Layer::Ability,
            attached_object,
            grant_delayed_ability,
            "Equipped creature has \"Whenever this creature attacks, you gain 1 life.\"",
        ),
    ],
    ..oracle::WHITE_MAGES_STAFF
};

pub static EXTRA: &[&CardDef] = &[&HERO_1_1_COLORLESS];
