use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const REMOVE_LORE: Question = Question::new(
    "RemoveLoreCounter",
    "remove a lore counter from it?",
    "Garnet gets a +1/+1 counter for each lore counter removed this way",
    "Remove a lore counter",
    "Leave it",
);

/// "Whenever Garnet attacks, you may remove a lore counter from each of any number of Sagas you control. Put a +1/+1
/// counter on Garnet for each lore counter removed this way." The Sagas are chosen as the ability resolves (CR 608.2d, not
/// targets): each Saga is offered (every Saga has a lore counter as it enters, CR 714.3a), asked of its controller in turn with the Saga as the question's
/// source, and "any number" is the ones answered yes. Then one counter comes off each chosen Saga (CR 122.1) and Garnet gets
/// a +1/+1 counter for each that came off (a Saga that is gone by then gives none: `remove_counters` says how many came
/// off). Removing lore counters triggers no chapter (CR 714.2b), and the Saga is sacrificed only if it still has as many
/// as its last chapter once state-based actions are checked (CR 714.4).
fn remove_lore_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut chosen = Vec::new();
    for saga in g.permanents_of(item.controller) {
        if g.chars(saga).has_subtype(Subtype::Saga) && effects::ask_may(g, item.controller, saga, Purpose::Ask(&REMOVE_LORE))? {
            chosen.push(saga);
        }
    }
    let removed: i32 = chosen.into_iter().map(|saga| g.remove_counters(saga, CounterKind::Lore, 1)).sum();
    if removed > 0 {
        g.add_counters(item.object, CounterKind::PlusOne,(removed) as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        remove_lore_counters,
        "Whenever Garnet attacks, you may remove a lore counter from each of any number of Sagas you control. Put a +1/+1 counter on Garnet for each lore counter removed this way.",
    )],
    ..oracle::GARNET_PRINCESS_OF_ALEXANDRIA
};
