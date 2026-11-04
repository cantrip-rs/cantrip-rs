use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::{HERO, JOB_SELECT};
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_mechanics::{effects, lands, statics};
use cantrip_oracle as oracle;

/// Job select (CR 702.182) and "Equipped creature has 'Whenever you cast a noncreature spell, this creature gets +2/+0 until end
/// of turn' and is a Wizard in addition to its other types" (CR 301.5, 613.1d, 613.1f): the type and the granted triggered
/// ability go with the Equipment, and the equipped creature is the one that gets the +2/+0.
pub static CARD: CardDef = CardDef {
    abilities: &[JOB_SELECT, equip(mana("{3}"), "Equip {3}")],
    delayed: &[triggered(
        NONCREATURE_SPELL_YOU_CAST,
        effects::this_gets::<2, 0>,
        "Whenever you cast a noncreature spell, this creature gets +2/+0 until end of turn.",
    )],
    statics: &[
        static_ability(Layer::Type, attached_object, statics::also_a::<{ lands::sub(Subtype::Wizard) }>, "Equipped creature is a Wizard in addition to its other types."),
        static_ability(
            Layer::Ability,
            attached_object,
            statics::grant_delayed_ability,
            "Equipped creature has \"Whenever you cast a noncreature spell, this creature gets +2/+0 until end of turn.\"",
        )
        .continues(),
    ],
    ..oracle::RED_MAGES_RAPIER
};

pub static EXTRA: &[&CardDef] = &[&HERO];
