//! Effects changing blocking relationships, distinct from declaring blockers.
use cantrip_api::*;
use crate::{Cx, layers::live, ops};

fn had_first_strike(cx: &Cx, creature: ObjectId) -> bool {
    cx.g.log.iter().rev()
        .take_while(|r| !matches!(r.event, Event::StepBegan { step: Step::BeginCombat } | Event::TurnBegan { .. }))
        .any(|r| matches!(r.event, Event::CombatFirstStrike { creature: id } if id == creature))
}

pub(crate) fn change_blocks(cx: &mut Cx, removed: &[ObjectId], ending: &[(ObjectId, ObjectId)], starting: &[(ObjectId, ObjectId)]) {
    if removed.is_empty() && ending.is_empty() && starting.is_empty() { return; }
    let before = cx.s().blocks.clone();
    ops::remove_from_combat(cx, removed);
    let mut blocks: Vec<_> = cx.s().blocks.iter().copied()
        .filter(|b| !ending.contains(&(b.blocker, b.attacker))).collect();
    let mut blockers = cx.s().blockers.clone();
    let mut attackers = cx.s().attackers.clone();
    let mut events = Vec::new();
    for &(blocker, attacker) in starting {
        // CR 506.3g: a battle cannot become a blocking creature. No declaration-time tests (untapped, evasion, costs)
        // here: this is the effect's instruction, after any explicit legality condition in the recipe.
        if !live(cx.s(), blocker) { continue; }
        let chars = cx.chars(blocker);
        if !chars.is(CardType::Creature) || chars.is(CardType::Battle) || attackers.iter().any(|a| a.id == blocker) {
            continue;
        }
        let Some(a) = attackers.iter().find(|a| a.id == attacker) else { continue };
        if a.defending != cx.s().obj(blocker).controller { continue; }
        let band = a.band;
        let group: Vec<_> = attackers.iter().filter(|a| a.id == attacker || (band != 0 && a.band == band)).map(|a| a.id).collect();
        let first_strike = before.iter().any(|b| b.blocker == blocker && b.first_strike) || had_first_strike(cx, blocker);
        for attacker in group {
            if blocks.iter().any(|b| b.blocker == blocker && b.attacker == attacker) { continue; }
            let first = !blockers.contains(&blocker);
            if first { blockers.push(blocker); }
            blocks.push(Block { blocker, attacker, first_strike });
            // An end/add pair is unchanged unless the creature actually left combat. Jarkeld preserves a shared
            // blocker; Sorrow's Path removes and reassigns it, producing fresh blocking events.
            if removed.contains(&blocker) || !before.iter().any(|b| b.blocker == blocker && b.attacker == attacker) {
                events.push(Event::Blocks { blocker, attacker, declared: first });
            }
            let a = attackers.iter_mut().find(|a| a.id == attacker).unwrap();
            if !a.blocked {
                a.blocked = true;
                events.push(Event::BecameBlocked { attacker });
            }
        }
    }
    if attackers != cx.s().attackers { cx.st().set_attackers(attackers); }
    if blocks != cx.s().blocks { cx.st().set_blocks(blocks); }
    if blockers != cx.s().blockers { cx.st().set_blockers(blockers); }
    if events.is_empty() { cx.state_changed(); } else { cx.emit_batch(&events, None); }
}
