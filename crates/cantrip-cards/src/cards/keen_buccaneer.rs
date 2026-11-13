use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw a card, then discard a card. Put a +1/+1 counter on this creature." (CR 701.9, 122.1): the counter goes on the
/// source only if it is still on the battlefield (CR 400.7), and is put on even when the hand was empty for the discard.
fn loot_and_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::loot(g, item)?;
    effects::plus_one_counter_on_this(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{U}"),
        loot_and_counter,
        "Exhaust \u{2014} {1}{U}: Draw a card, then discard a card. Put a +1/+1 counter on this creature. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::KEEN_BUCCANEER
};
