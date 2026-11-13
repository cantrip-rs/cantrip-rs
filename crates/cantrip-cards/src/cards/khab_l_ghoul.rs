use cantrip_api::*;
use cantrip_mechanics::{history, triggers::EACH_END_STEP};
use cantrip_oracle as oracle;

/// Count creatures that died this turn as the ability resolves (CR 700.4, 608.2h).
fn counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = history::creatures_died_this_turn(g, None) as i32;
    g.add_counters(item.object, CounterKind::PlusOne, count);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(EACH_END_STEP, counters, "At the beginning of each end step, put a +1/+1 counter on this creature for each creature that died this turn."),
    ],
    ..oracle::KHAB_L_GHOUL
};
