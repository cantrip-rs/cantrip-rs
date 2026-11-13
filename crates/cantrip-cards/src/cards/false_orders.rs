use cantrip_api::*;
use cantrip_mechanics::{combat, effects, targets};
use cantrip_oracle as oracle;

/// Cast only during the declare blockers step (CR 601.3).
fn during_blockers(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::DeclareBlockers
}

/// Remove the creature, unblock creatures blocked only by it, then optionally have it block (CR 509.1h).
fn false_orders(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(blocker) = targets::object(g, item, 0) else { return Ok(()) };
    combat::remove_from_combat_and_unblock(g, blocker);
    g.end_sentence();
    if effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        let attackers: Vec<ObjectId> = g.state().attackers.iter().map(|a| a.id).collect();
        combat::choose_attacker_to_block(g, &Asker::new(item.object, item.controller), Some(blocker), &attackers)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(targets::spell_creature_defending_player_controls)], false_orders),
    cast_only_if: Some(during_blockers),
    ..oracle::FALSE_ORDERS
};
