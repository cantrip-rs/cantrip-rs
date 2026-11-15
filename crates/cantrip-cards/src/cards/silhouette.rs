use cantrip_api::*;
use cantrip_mechanics::{prevention, targets};
use cantrip_oracle as oracle;

/// "If a spell or ability that targets that creature would cause a source to deal damage to that creature this turn, prevent
/// that damage" (CR 615.1): damage to the creature the effect is about (its recipient, which the core matches: the effect's
/// `to`) that is the effect of a spell or ability that targets that creature (CR 120.2b) - the spell or ability whose
/// instructions are being carried out as it is dealt (CR 608.2c, `Query::instructing`), whatever the source it has deal the
/// damage (its own, a fighting creature's, an ability's source) and whatever else it targets or damages. Combat damage
/// (CR 120.2a) is no spell's or ability's; what a mana ability or a replacement or prevention effect does of its own (CR 605.3b,
/// 615.5) is not the resolving spell's; and damage a spell or ability that does not target the creature causes, redirected
/// to it or not (CR 614.9), is not caused by one that does. A creature that has left the battlefield is a new object
/// (CR 400.7), which neither the effect nor a spell that targeted the old one is about.
fn caused_by_one_that_targets_it(q: &dyn Query, _: &Asker, creature: ObjectId, _: &DamageEvent) -> bool {
    q.instructing().is_some_and(|item| q.state().targets(item).contains(&Target::Object(creature)))
}

/// "Choose target creature. If a spell or ability that targets that creature would cause a source to deal damage to that
/// creature this turn, prevent that damage" (CR 615.1, 611.2a): the effect about the creature, if it is still a legal target
/// (CR 608.2b), until the turn ends (CR 514.2).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    prevention::this_turn(g, item, Target::Object(creature), 0, Shield::All);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents the damage a spell or ability that targets it would cause to be dealt to it",
        damage: "damage",
        of_it: Some("to it as the effect of a spell or ability that targets it"),
        applies: caused_by_one_that_targets_it,
        portion: None,
        then: None,
        gain_prevented: false,
        text: "If a spell or ability that targets that creature would cause a source to deal damage to that creature this turn, prevent that damage.",
    }],
    ..oracle::SILHOUETTE
};
