use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Soul Immolation deals X damage to each opponent and each creature they control" (CR 120.3, 107.3i): X the spell's, announced
/// with the additional cost, dealt to all of them together (CR 120.2).
fn damage_each_opponent_and_their_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hits: Vec<(ObjectId, Target, u16)> = Vec::new();
    for opponent in g.state().opponents(item.controller).collect::<Vec<_>>() {
        hits.push((item.object, Target::Player(opponent), item.x));
        for creature in g.permanents_of(opponent) {
            if g.chars(creature).is(CardType::Creature) {
                hits.push((item.object, Target::Object(creature), item.x));
            }
        }
    }
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "As an additional cost to cast this spell, blight X": X is announced though the mana cost has no {X} (CR 601.2b).
    additional_cost: Some(Cost { variable: Some(VariableCost::BlightX), ..Cost::NONE }),
    spell: spell(&[], damage_each_opponent_and_their_creatures),
    ..oracle::SOUL_IMMOLATION
};
