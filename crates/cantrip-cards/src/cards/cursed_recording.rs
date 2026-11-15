use cantrip_api::*;
use cantrip_mechanics::{copy, delayed, triggers};
use cantrip_oracle as oracle;

/// "Put a time counter on this artifact. Then if there are seven or more time counters on it, remove those counters and it deals 20
/// damage to you" (CR 122.1, 608.2c, 120.3): the counter goes on first, and the count is read after it, as the "Then" is its own
/// sentence. The artifact deals the damage even if it has left the battlefield by then (CR 608.2h); a counter cannot go on it
/// then, so the count it last had (never seven or more, as it would have been removed) is what is read.
fn tick(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let recording = item.object;
    g.add_counters(recording, CounterKind::Time, 1);
    g.end_sentence();
    let counters = g.obj(recording).counters(CounterKind::Time);
    if counters >= 7 {
        g.remove_counters(recording, CounterKind::Time, counters);
        g.damage(recording, Target::Player(item.controller), 20);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::INSTANT_OR_SORCERY_YOU_CAST,
            tick,
            "Whenever you cast an instant or sorcery spell, put a time counter on this artifact. Then if there are seven or more time counters on it, remove those counters and it deals 20 damage to you.",
        ),
        ability(
            TAP,
            delayed::watch_for_the_next,
            "{T}: When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.",
        ),
    ],
    delayed: &[copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST],
    ..oracle::CURSED_RECORDING
};
