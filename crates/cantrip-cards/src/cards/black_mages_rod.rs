use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::job_select::{HERO, JOB_SELECT};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_oracle as oracle;

/// "This creature deals 1 damage to each opponent" (CR 120.3): the equipped creature is the source, so its own damage
/// modifiers and protection-like effects see it; each opponent takes one damage event, together (CR 120.2).
fn ping(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::damage_each_opponent(g, item, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[JOB_SELECT, equip(mana("{3}"), "Equip {3}")],
    delayed: &[triggered(NONCREATURE_SPELL_YOU_CAST, ping, "Whenever you cast a noncreature spell, this creature deals 1 damage to each opponent.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Equipped creature gets +1/+0."),
        static_ability(Layer::Type, attached_object, cantrip_mechanics::statics::also_a::<{ cantrip_mechanics::lands::sub(Subtype::Wizard) }>, "Equipped creature is a Wizard in addition to its other types."),
        static_ability(Layer::Ability, attached_object, cantrip_mechanics::statics::grant_delayed_ability, "Equipped creature has \"Whenever you cast a noncreature spell, this creature deals 1 damage to each opponent.\"").continues(),
    ],
    ..oracle::BLACK_MAGES_ROD
};

pub static EXTRA: &[&CardDef] = &[&HERO];
