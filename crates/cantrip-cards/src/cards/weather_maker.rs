use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets, triggers::LANDFALL};
use cantrip_oracle as oracle;

/// "Put a charge counter on this artifact."
fn charge(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Charge, 1);
    Ok(())
}

/// "It deals 3 damage to any target."
fn shock(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(LANDFALL, charge, "Landfall \u{2014} Whenever a land you control enters, put a charge counter on this artifact."),
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        mana_ability(
            TAP.and(remove_counters(CounterKind::Charge, 2)),
            add_n(Mana::C, 2),
            "{T}, Remove two charge counters from this artifact: Add {C}{C}.",
        ),
        ability(TAP.and(remove_counters(CounterKind::Charge, 3)), shock, "{T}, Remove three charge counters from this artifact: It deals 3 damage to any target.")
            .targets(&[targets::ANY]),
    ],
    ..oracle::WEATHER_MAKER
};
