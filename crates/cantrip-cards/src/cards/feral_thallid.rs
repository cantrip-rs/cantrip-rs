use cantrip_api::*;
use cantrip_mechanics::{regenerate, triggers};
use cantrip_oracle as oracle;

/// Put one spore counter on this creature (CR 122.1).
fn put_spore_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Spore, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            put_spore_counter,
            "At the beginning of your upkeep, put a spore counter on this creature.",
        ),
        ability(
            remove_counters(CounterKind::Spore, 3),
            regenerate::this_effect,
            "Remove three spore counters from this creature: Regenerate this creature.",
        ),
    ],
    ..oracle::FERAL_THALLID
};
