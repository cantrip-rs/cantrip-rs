use cantrip_api::*;
use cantrip_mechanics::{pump, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, it gets +1/+1 until end of turn for each other creature you control" (CR 611.2a): the
/// creatures counted as this resolves, itself not among them. Nothing happens if it is no longer on the battlefield.
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others = g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).is(CardType::Creature)).count() as i16;
    pump(g, item.object, item.object, others, others);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, grow, "Whenever this creature attacks, it gets +1/+1 until end of turn for each other creature you control.")],
    ..oracle::EAGLE_OF_THE_GREAT_SHELF
};
