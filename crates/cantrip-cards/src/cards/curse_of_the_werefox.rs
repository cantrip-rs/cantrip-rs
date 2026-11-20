use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Create a Monster Role token attached to target creature you control. When you do, that creature fights up to one target creature
/// you don't control." The Role is attached by the effect, not targeted; one that could not enchant the creature is not created
/// (CR 303.4i). The second sentence is a reflexive triggered ability (CR 603.12) that triggers only if the Role really was created,
/// carrying the creature as its subject; the first sentence does nothing if its target is no longer legal (CR 608.2b).
fn create_monster_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if !tokens::create_role(g, &tokens::MONSTER_ROLE, item.controller, creature)?.is_empty() {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::Object(creature));
    }
    Ok(())
}

/// "That creature fights up to one target creature you don't control": the creature the Role was attached to fights the chosen
/// target (CR 701.14a), judged as this resolves (CR 608.2b); with none chosen, or the target gone, there is no fight, and the fight
/// does nothing unless both are still creatures on the battlefield (CR 701.14b).
fn creature_fights_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Target::Object(creature), Some(prey)) = (item.subject, targets::object(g, item, 0)) {
        g.fight(creature, prey);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], create_monster_role),
    delayed: &[delayed_trigger(creature_fights_target, "When you do, that creature fights up to one target creature you don't control.")
        .targets(&[targets::CREATURE_YOU_DONT_CONTROL.optional()])],
    ..oracle::CURSE_OF_THE_WEREFOX
};

pub static EXTRA: &[&CardDef] = &[&tokens::MONSTER_ROLE];
