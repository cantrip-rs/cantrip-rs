use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "If Loot's power is negative, he assigns combat damage as though his power were positive." (CR 510.1a, 609.4.)
fn loot_assigns_absolute_power(_: &dyn Query, asker: &Asker, creature: ObjectId) -> bool {
    asker.source == creature
}

/// "Loot gets -2/-0 until end of turn." (CR 611.2a.)
fn shrink_loot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    pump(g, item.object, item.object, -2, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(1, targets::filters::another_creature_or_planeswalker),
        shrink_loot,
        "Threshold — Sacrifice another creature or planeswalker: Loot gets -2/-0 until end of turn. Activate only if there are seven or more cards in your graveyard.",
    )
    .condition(cantrip_mechanics::you_have_threshold)],
    damage_by_absolute_power: &[loot_assigns_absolute_power],
    ..oracle::LOOT_THE_ANOMALY
};
