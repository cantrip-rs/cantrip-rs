use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Put an ice counter on this enchantment." (CR 122.6).
fn put_ice_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Ice, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{3}"), put_ice_counter, "{3}: Put an ice counter on this enchantment."),
        mana_ability(remove_counters(CounterKind::Ice, 1), add(Mana::C),
            "Remove an ice counter from this enchantment: Add {C}."),
    ],
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::Ice as usize }>),
    ..oracle::ICEBERG
};
