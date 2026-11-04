use cantrip_api::*;
use cantrip_mechanics::{mana, triggers};
use cantrip_oracle as oracle;

/// Put a storage counter on this land (CR 122.1, 603.4).
fn put_storage_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Storage, 1);
    Ok(())
}

/// Add one green mana for each storage counter removed as the cost (CR 605.1a, 106.3).
fn add_green_for_counters(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::G, g.mana_ability_x().unwrap_or(0));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            put_storage_counter,
            "At the beginning of your upkeep, if this land is tapped, put a storage counter on it.",
        )
        .condition(triggers::this_is_tapped),
        mana_ability(
            TAP.and(Cost {
                variable: Some(VariableCost::RemoveAnyNumberOfCounters(CounterKind::Storage)),
                ..Cost::NONE
            }),
            add_by(add_green_for_counters),
            "{T}, Remove any number of storage counters from this land: Add {G} for each storage counter removed this way.",
        )
        .x_is_mana_amount(),
    ],
    enters: Some(mana::enters_tapped),
    may_not_untap: true,
    ..oracle::HOLLOW_TREES
};
