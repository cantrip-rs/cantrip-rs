use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_LEAVES;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Put its +1/+1 counters on target creature you control": as many +1/+1 counters as it had as it last was on the battlefield
/// (CR 603.10a, 608.2h), counters of other kinds staying behind (only "+1/+1 counters" are printed). The target is chosen as the
/// ability goes on the stack, so the Captain itself, already gone, is never a choice (CR 115.1); with no other creature of yours
/// the ability is removed (CR 603.3d). A target that became illegal gets nothing (CR 608.2b).
fn put_its_plus_one_counters_on_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let counters = g.obj(item.object).counters(CounterKind::PlusOne);
    g.add_counters(creature, CounterKind::PlusOne,(counters) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_a_counter::<{ CounterKind::PlusOne as usize }>),
    abilities: &[triggered(
        THIS_LEAVES,
        put_its_plus_one_counters_on_target,
        "When this creature leaves the battlefield, put its +1/+1 counters on target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::SELFLESS_POLICE_CAPTAIN
};
