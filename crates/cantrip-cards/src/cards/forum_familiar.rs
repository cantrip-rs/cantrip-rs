use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_TURNED_FACE_UP;
use cantrip_mechanics::{disguise, effects, targets};
use cantrip_oracle as oracle;

/// "Return another target permanent you control to its owner's hand and put a +1/+1 counter on this creature" (CR 400.3, 122.1): two
/// sentences of one ability; if the permanent is no longer a legal target as it resolves, nothing happens (CR 608.2b), and the counter
/// goes on the Familiar only while it is still the permanent that was turned up (CR 400.7).
fn bounce_and_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_hand(g, item)?;
    effects::plus_one_counter_on_this(g, item)
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{1}{W}")), "Disguise {1}{W}")],
    abilities: &[triggered(
        THIS_TURNED_FACE_UP,
        bounce_and_grow,
        "When this creature is turned face up, return another target permanent you control to its owner's hand and put a +1/+1 counter on this creature.",
    )
    .targets(&[targets::ANOTHER_PERMANENT_YOU_CONTROL])],
    ..oracle::FORUM_FAMILIAR
};
