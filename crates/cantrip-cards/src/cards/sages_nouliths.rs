use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::{HERO, JOB_SELECT};
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[JOB_SELECT, equip(mana("{3}"), "Hagneia \u{2014} Equip {3}")],
    // "Whenever this creature attacks, untap target attacking creature" (CR 603.2, 113.6): the equipped creature's own ability.
    delayed: &[triggered(triggers::THIS_ATTACKS, effects::untap_targets, "Whenever this creature attacks, untap target attacking creature.")
        .targets(&[targets::ATTACKING_CREATURE])],
    statics: &[
        static_ability(Layer::Type, attached_object, cantrip_mechanics::statics::also_a::<{ cantrip_mechanics::lands::sub(Subtype::Cleric) }>, "Equipped creature is a Cleric in addition to its other types."),
        static_ability(Layer::Ability, attached_object, cantrip_mechanics::statics::grant_delayed_ability, "Equipped creature has \"Whenever this creature attacks, untap target attacking creature.\"").continues(),
        static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<1, 0>, "Equipped creature gets +1/+0.").continues(),
    ],
    ..oracle::SAGES_NOULITHS
};

pub static EXTRA: &[&CardDef] = &[&HERO];
