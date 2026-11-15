use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Tap the creature, then add colorless mana equal to its mana value (CR 701.26a, 106.3).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    // A legal target is untapped, so tapping it succeeds (CR 701.26a).
    g.tap(creature);
    g.end_sentence();
    g.add_mana(item.controller, Mana::C, g.mana_value(creature));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::UNTAPPED_CREATURE_YOU_CONTROL], resolve),
    ..oracle::ENERGY_TAP
};
