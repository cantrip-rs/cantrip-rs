use cantrip_api::*;
use cantrip_mechanics::{prevention, triggers};
use cantrip_oracle as oracle;

/// The trigger puts one spore counter on its source (CR 122.1, 608.2).
fn put_spore_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters_together(&[(item.object, CounterKind::Spore, 1)]);
    Ok(())
}

/// The activated ability creates a prevention effect through this turn (CR 615.1, 611.2a).
fn prevent_combat_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::all_combat_damage_this_turn(g, item, 0);
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
            prevent_combat_damage,
            "Remove three spore counters from this creature: Prevent all combat damage that would be dealt this turn.",
        ),
    ],
    granted_preventions: &[prevention::COMBAT_DAMAGE.text("Prevent all combat damage that would be dealt this turn.")],
    ..oracle::SPORE_FLOWER
};
