use cantrip_api::*;
use cantrip_mechanics::{damage_target, flashback, targets};
use cantrip_oracle as oracle;

/// "Duel Tactics deals 1 damage to target creature. It can't block this turn." The target is illegal as it resolves (CR 608.2b):
/// neither sentence is done. "It" is the creature the damage was dealt to, so one that died of the damage (CR 400.7) is not given
/// the restriction, and one that the damage was prevented from still is (the second sentence does not depend on the first).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    damage_target(g, item, 0, 1);
    g.end_sentence();
    g.add_effect(item.object, creature, EffectKind::CantBlock, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ways: &[flashback::way(cost("{1}{R}"), "Flashback {1}{R}")],
    ..oracle::DUEL_TACTICS
};
