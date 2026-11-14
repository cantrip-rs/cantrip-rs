use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers};
use cantrip_oracle as oracle;

/// "Put a carrion counter on this creature" (CR 122.1).
fn put_carrion(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Carrion, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::EACH_END_STEP,
            put_carrion,
            "At the beginning of each end step, if a creature died this turn, put a carrion counter on this creature.",
        ).condition(history::a_creature_died_this_turn),
        ability(
            remove_counters(CounterKind::Carrion, 2),
            effects::this_gets::<1, 1>,
            "Remove two carrion counters from this creature: This creature gets +1/+1 until end of turn.",
        ),
    ],
    ..oracle::OSAI_VULTURES
};
