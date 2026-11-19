use cantrip_api::*;
use cantrip_mechanics::effects::{self, ask_may};
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_oracle as oracle;

const ASK: Question =
    Question::new("RemoveACounter", "remove a +1/+1 counter from it?", "If you do, draw a card", "Remove a counter", "Keep it");

/// The intervening "if this creature has one or more +1/+1 counters on it" (CR 603.4): checked as the trigger would go on the stack and
/// again as it resolves.
fn has_a_plus_one_counter(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).counters(CounterKind::PlusOne) > 0
}

/// "You may remove a +1/+1 counter from it. If you do, draw a card" (CR 122.1, 608.2c, 121.1): the counter is removed as the ability
/// resolves. The trigger's "if" is checked again as it resolves, so a counter is there to remove once the player has agreed.
fn remove_a_counter_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let removed = g.remove_counters(item.object, CounterKind::PlusOne, 1);
    g.end_sentence();
    if removed > 0 {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

/// "Skulk" (a core keyword). "This creature enters with X +1/+1 counters on it" (CR 614.1c, 107.3).
pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::PlusOne as usize }>),
    abilities: &[triggered(
        YOUR_UPKEEP,
        remove_a_counter_to_draw,
        "At the beginning of your upkeep, if this creature has one or more +1/+1 counters on it, you may remove a +1/+1 counter from it. If you do, draw a card.",
    )
    .condition(has_a_plus_one_counter)],
    ..oracle::INGENIOUS_PRODIGY
};
