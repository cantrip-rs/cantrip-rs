use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Up to one target artifact, creature, or planeswalker an opponent controls" (CR 115.1, 601.2c): the slot may be left unchosen.
fn artifact_creature_or_planeswalker_an_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::ARTIFACT_OR_CREATURE_AN_OPPONENT_CONTROLS.filter)(q, asker, t) || (targets::CREATURE_OR_PLANESWALKER_AN_OPPONENT_CONTROLS.filter)(q, asker, t)
}

/// "Up to one target artifact, creature, or planeswalker an opponent controls loses all abilities until your next turn. If it's a
/// creature, it also has base power and toughness 2/2 until your next turn." (CR 611.2a, 613.4b): both effects last until
/// the start of your next turn; the 2/2 is applied only if the target is a creature as the ability resolves.
fn silence(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let duration = Duration::UntilNextTurnOf(item.controller);
    instruction(g, |g| g.add_effect(item.object, target, EffectKind::LoseAllAbilities, duration));
    g.end_sentence();
    if g.chars(target).is(CardType::Creature) {
        instruction(g, |g| g.add_effect(item.object, target, EffectKind::SetPt(2, 2), duration));
    }
    Ok(())
}

/// "This creature can't be blocked by creatures with power 2 or greater" (CR 509.1b): the blocker's current power, checked as
/// blockers are declared.
fn power_two_or_greater_cant_block(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || q.chars(blocker).power.unwrap_or(0) < 2
}

pub static CARD: CardDef = CardDef {
    block_rules: &[power_two_or_greater_cant_block],
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        silence,
        "Whenever this creature attacks, up to one target artifact, creature, or planeswalker an opponent controls loses all abilities until your next turn. If it's a creature, it also has base power and toughness 2/2 until your next turn.",
    )
    .targets(&[up_to_one(artifact_creature_or_planeswalker_an_opponent_controls)])],
    ..oracle::AZURE_BEASTBINDER
};
