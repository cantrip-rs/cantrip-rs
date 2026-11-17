use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on each creature you control that entered this turn." (CR 122.1, 608.2, 400.7): the creatures its controller
/// controls as it resolves that came onto the battlefield this turn (this creature among them if it did), all at once (CR 101.4). Each is
/// the object that move made, so one that left and came back is a new one that did enter this turn.
fn counters_on_creatures_that_entered_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let entered: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature) && history::entered_this_turn(g.history(), id))
        .collect();
    effects::counters_on_each(g, &entered, CounterKind::PlusOne, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}, {T}"),
        counters_on_creatures_that_entered_this_turn,
        "{1}, {T}: Put a +1/+1 counter on each creature you control that entered this turn.",
    )],
    ..oracle::RAUCOUS_ENTERTAINER
};
