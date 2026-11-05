use cantrip_api::*;
use cantrip_mechanics::{history, regenerate, triggers::EACH_END_STEP};
use cantrip_oracle as oracle;

/// Count creatures that died this turn as the end-step ability resolves (CR 700.4, 608.2h).
fn corpses(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = i64::try_from(history::creatures_died_this_turn(g, None)).expect("death count fits the native event domain");
    g.add_counters(item.object, CounterKind::Corpse, count);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(EACH_END_STEP, corpses, "At the beginning of each end step, put a corpse counter on this creature for each creature that died this turn."),
        ability(remove_counters(CounterKind::Corpse, 1), regenerate::this_effect, "Remove a corpse counter from this creature: Regenerate this creature."),
    ],
    ..oracle::SCAVENGING_GHOUL
};
