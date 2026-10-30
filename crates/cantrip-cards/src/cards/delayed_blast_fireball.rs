use cantrip_api::*;
use cantrip_mechanics::effects::damage_each_opponent_and_each_creature_they_control;
use cantrip_mechanics::foretell;
use cantrip_oracle as oracle;

/// "Delayed Blast Fireball deals 2 damage to each opponent and each creature they control. If this spell was cast from
/// exile, it deals 5 damage to each opponent and each creature they control instead." Where it was cast from is the zone
/// the cast record says (CR 601.2a): any exile counts, a foretold card or one an effect let its caster cast from there.
fn fireball(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let from_exile = g.history().cast_of(item.object).is_some_and(|c| c.from == ZoneKind::Exile);
    let amount = if from_exile { 5 } else { 2 };
    damage_each_opponent_and_each_creature_they_control(g, item, amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], fireball),
    abilities: &[foretell::ABILITY],
    foretell: Some(mana("{4}{R}{R}")),
    ..oracle::DELAYED_BLAST_FIREBALL
};
