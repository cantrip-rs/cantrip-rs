use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::saddle;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each other creature you control. You gain 1 life for each of those creatures." (CR 608.2c): the
/// creatures are the ones the controller has as it resolves, all counted for the life even when a counter can't be put on one
/// (the 2024-04-12 ruling).
fn counters_on_the_others_then_gain_life_for_each(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).is(CardType::Creature)).collect();
    effects::counters_on_each(g, &others, CounterKind::PlusOne, 1);
    effects::gain_life(g, item, others.len() as i32)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            counters_on_the_others_then_gain_life_for_each,
            "Whenever this creature attacks while saddled, put a +1/+1 counter on each other creature you control. You gain 1 life for each of those creatures.",
        ),
        saddle::ability(2, "Saddle 2 (Tap any number of other creatures you control with total power 2 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::BOUNDING_FELIDAR
};
