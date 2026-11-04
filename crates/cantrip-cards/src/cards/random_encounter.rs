use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, flashback};
use cantrip_oracle as oracle;

/// "Shuffle your library, then mill four cards. Put each creature card milled this way onto the battlefield. They gain haste. At
/// the beginning of the next end step, return those creatures to their owner's hand." (CR 701.24, 701.17, 603.7c): the creature
/// cards milled by this spell, the ones still in the graveyard, enter together under its controller's control, gain haste with no
/// duration (CR 611.2a), and a delayed ability about exactly them returns them.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.shuffle(item.controller);
    let milled = g.mill(item.controller, 4);
    let creatures: Vec<(ObjectId, PlayerId)> = milled
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .map(|id| (id, item.controller))
        .collect();
    let entered = g.put_all_onto_battlefield(&creatures)?;
    instruction(g, |g| {
        for &id in &entered {
            effects::gain_haste(g, item.object, id);
        }
    });
    delayed::at_next_end_step_about(g, item, 0, &entered);
    Ok(())
}

/// "Return those creatures to their owner's hand" (CR 603.7c, 400.7): the carried objects that are still on the battlefield.
fn return_those_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let entered = g.event_objects(item).to_vec();
    g.return_to_hand(&entered);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[flashback::way(cost("{6}{R}{R}"), "Flashback {6}{R}{R}")],
    delayed: &[delayed_trigger(
        return_those_creatures,
        "At the beginning of the next end step, return those creatures to their owner's hand.",
    )],
    ..oracle::RANDOM_ENCOUNTER
};
