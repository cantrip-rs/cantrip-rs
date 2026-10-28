use cantrip_api::*;
use cantrip_mechanics::{power_up, statics};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter and a double strike counter on Quicksilver" (CR 122.1b): both are put on together, as one
/// instruction; on nothing if he has left the battlefield (CR 400.7).
fn counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters_together(&[(item.object, CounterKind::PlusOne, 1), (item.object, CounterKind::DoubleStrike, 1)]);
    Ok(())
}

/// "If Quicksilver, Brash Blur is in your opening hand, you may begin the game with him on the battlefield" (CR 103.6); haste is a
/// core keyword.
pub static CARD: CardDef = CardDef {
    opening_hand: Some(statics::BEGIN_ON_THE_BATTLEFIELD),
    abilities: &[power_up::ability(
        cost("{4}{R}"),
        counters,
        "Power-up \u{2014} {4}{R}: Put a +1/+1 counter and a double strike counter on Quicksilver. (Activate each power-up ability only once. Reduce the cost by his mana cost if he entered this turn.)",
    )],
    ..oracle::QUICKSILVER_BRASH_BLUR
};
