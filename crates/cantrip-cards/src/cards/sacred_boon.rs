use cantrip_api::*;
use cantrip_mechanics::{delayed, prevention, targets};
use cantrip_oracle as oracle;

/// "Prevent the next 3 damage that would be dealt to target creature this turn. At the beginning of the next end step, put a
/// +0/+1 counter on that creature for each 1 damage prevented this way." (CR 615.7, 603.7): a shield of 3 on the creature until
/// the turn ends, then (CR 608.2c) a delayed ability about that creature, set up whether or not the shield ever prevents
/// anything. Its source is this spell, the source of the shield too (CR 603.7d).
fn shield_then_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    prevention::next_damage_to_target(g, item, 0, 0, 3);
    g.end_sentence();
    delayed::at_next_end_step(g, item, 0, Target::Object(creature));
    Ok(())
}

/// "Put a +0/+1 counter on that creature for each 1 damage prevented this way" (CR 615.7, 122.1, 608.2h): the damage this
/// spell's shield prevented, counted as the delayed ability resolves - each prevention the history records with the spell as
/// the effect's source (`Event::DamagePrevented::by`; a copy of the spell is another object with its own shield, CR 707.10) -
/// put on at once, as one event. "That creature" is the object it was: one that has left the battlefield is a new object and
/// gets none (CR 400.7, the counter verb).
fn counters_for_damage_prevented(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let spell = item.object;
    let prevented: u32 = g
        .history()
        .select(EventQuery::kind(EventMask::DAMAGE_PREVENTED).object(spell))
        .filter_map(|r| match r.event { Event::DamagePrevented { by, amount, .. } if by == spell => Some(amount), _ => None })
        .sum();
    g.add_counters(creature, CounterKind::Plus0Plus1, prevented as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], shield_then_counters),
    delayed: &[delayed_trigger(
        counters_for_damage_prevented,
        "At the beginning of the next end step, put a +0/+1 counter on that creature for each 1 damage prevented this way.",
    )],
    granted_preventions: &[prevention::ANY_DAMAGE.text("Prevent the next 3 damage that would be dealt to target creature this turn.")],
    ..oracle::SACRED_BOON
};
