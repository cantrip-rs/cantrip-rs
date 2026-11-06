use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// Put a storage counter on the source (CR 122.1, 400.7).
fn put_storage_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Storage, 1);
    Ok(())
}

/// Add colorless mana for the storage counters on this land (CR 106.3).
fn add_per_storage_counter(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let counters = g.obj(asker.source).counters(CounterKind::Storage);
    if counters > 0 {
        g.add_mana(asker.controller, Mana::C, counters as u32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP.and(exile_permanents(1, filters::creature)),
            put_storage_counter,
            "{T}, Exile a creature you control: Put a storage counter on this land.",
        ),
        mana_ability(
            TAP,
            add_by(add_per_storage_counter),
            "{T}: Add {C} for each storage counter on this land.",
        ),
    ],
    ..oracle::CITY_OF_SHADOWS
};
