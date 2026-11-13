use cantrip_api::*;
use cantrip_mechanics::{effects, hand};
use cantrip_oracle as oracle;

/// "Discard up to two cards, then draw that many cards. Put a +1/+1 counter on this creature." (CR 701.9, 121.1, 122.1): the
/// discard-and-draw is one instruction (and can be nothing at all: the ruling lets the counter come alone), and the counter
/// follows as the next sentence, on this creature only while it is still the object that activated the ability (CR 400.7).
fn loot_two_and_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::discard_up_to_then_draw(g, item.controller, item.object, 2)?;
    g.end_sentence();
    effects::plus_one_counter_on_this(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{R}"),
        loot_two_and_counter,
        "Exhaust \u{2014} {2}{R}: Discard up to two cards, then draw that many cards. Put a +1/+1 counter on this creature. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    ..oracle::GREASEWRENCH_GOBLIN
};
