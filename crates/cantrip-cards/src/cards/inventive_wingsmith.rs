use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "If you haven't cast a spell from your hand this turn and this creature doesn't have a flying counter on it" (CR 603.4): the
/// intervening "if" is true when the ability would trigger and again as it resolves. A spell cast from anywhere but your own
/// hand (the graveyard, exile, the library's top, another player's hand) doesn't count, nor does one an opponent cast, nor does
/// a copy (CR 707.10).
fn no_spell_from_your_hand_and_no_flying_counter(q: &dyn Query, asker: &Asker) -> bool {
    history::you_havent_cast_a_spell_from_your_hand(q, asker)
        && q.obj(asker.source).counters(CounterKind::Flying) == 0
}

/// "Put a flying counter on it" (CR 122.1b): on the creature as it is now; once it has left the battlefield it is another
/// object (CR 400.7) and gets none.
fn put_a_flying_counter_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Flying, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        put_a_flying_counter_on_this,
        "At the beginning of your end step, if you haven't cast a spell from your hand this turn and this creature doesn't have a flying counter on it, put a flying counter on it.",
    )
    .condition(no_spell_from_your_hand_and_no_flying_counter)],
    ..oracle::INVENTIVE_WINGSMITH
};
