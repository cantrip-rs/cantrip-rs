use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Destroy all creatures. You gain 1 life for each creature destroyed this way." (CR 701.8a, 119.3): the creatures on the battlefield as
/// it resolves, destroyed together, and the life for those that really were.
fn destroy_all_creatures_and_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    effects::destroy_all_and_gain_life_for_each_destroyed(g, item.controller, &creatures);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], destroy_all_creatures_and_gain_life),
    ..oracle::FUMIGATE
};
