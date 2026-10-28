use cantrip_api::*;
use cantrip_mechanics::amass::amass;
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_DIES};
use cantrip_mechanics::{effects, select, targets, tokens};
use cantrip_oracle as oracle;

/// "Another creature" (CR 701.21a): a creature permanent other than the object whose text asks.
fn another_creature(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| id != choice.source && q.chars(id).is(CardType::Creature))
}

/// The power of the permanent a sacrifice made `sacrificed` (the new object in the graveyard) out of, as it last existed on
/// the battlefield (CR 608.2h, the ruling): pumps, counters and copy effects included.
fn power_when_sacrificed(q: &dyn Query, sacrificed: ObjectId) -> i16 {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).this_turn())
        .find_map(|r| match r.event {
            Event::ZoneChange { old, new, .. } if new == sacrificed => Some(targets::power_or_last_known(q, old)),
            _ => None,
        })
        .unwrap_or(0)
}

/// "Whenever this creature attacks, you may sacrifice another creature. If you do, put a number of +1/+1 counters on this
/// creature equal to the sacrificed creature's power." The sacrifice is chosen as the ability resolves and may be declined
/// (CR 608.2d); the counters follow only a sacrifice that happened (CR 608.2c), their number the creature's power as it last
/// existed on the battlefield, none for a power of 0 or less (CR 107.1b). They go on this creature if it is still on the
/// battlefield; the sacrifice may not be itself.
fn sacrifice_for_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = Selection::new(select::up_to(1)).with(select::part(&(another_creature as SelectFn), 0, "")).called("other creature");
    let Some(&sacrificed) = effects::may_sacrifice(g, item.controller, item.object, rule)?.first() else { return Ok(()) };
    g.end_sentence();
    let power = power_when_sacrificed(g, sacrificed);
    if power > 0 {
        g.add_counters(item.object, CounterKind::PlusOne, power);
    }
    Ok(())
}

/// "When this creature dies, amass Goblins X, where X is this creature's power" (CR 701.47a, 608.2h): its power as it last
/// existed on the battlefield, none for a power of 0 or less. The Army is made even for X of 0 (CR 701.47a).
fn amass_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = targets::power_or_last_known(g, item.object).max(0);
    amass(g, item.controller, item.object, &tokens::GOBLIN_ARMY, Subtype::Goblin, x)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            sacrifice_for_counters,
            "Whenever this creature attacks, you may sacrifice another creature. If you do, put a number of +1/+1 counters on this creature equal to the sacrificed creature's power.",
        ),
        triggered(THIS_DIES, amass_its_power, "When this creature dies, amass Goblins X, where X is this creature's power."),
    ],
    ..oracle::RHOVANION_RAMPAGER
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];
