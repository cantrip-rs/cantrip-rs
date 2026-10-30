use cantrip_api::*;
use cantrip_mechanics::effects::damage_each_opponent_and_each_creature_they_control;
use cantrip_oracle as oracle;

/// "Soul Immolation deals X damage to each opponent and each creature they control" (CR 120.3, 107.3i): X the spell's, announced
/// with the additional cost, dealt to all of them together (CR 120.2).
fn damage_each_opponent_and_their_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_each_opponent_and_each_creature_they_control(g, item, item.x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "As an additional cost to cast this spell, blight X": X is announced though the mana cost has no {X} (CR 601.2b).
    additional_cost: Some(Cost { variable: Some(VariableCost::BlightX), ..Cost::NONE }),
    spell: spell(&[], damage_each_opponent_and_their_creatures),
    ..oracle::SOUL_IMMOLATION
};
