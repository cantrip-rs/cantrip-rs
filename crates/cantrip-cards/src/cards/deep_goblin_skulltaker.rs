use cantrip_api::*;
use cantrip_mechanics::{effects, history, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your end step, if you descended this turn, put a +1/+1 counter on this creature" (CR 603.4, 700.11): the
/// intervening "if" is checked as the end step begins and again as the ability resolves. Menace comes from the card's own keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        effects::plus_one_counter_on_this,
        "At the beginning of your end step, if you descended this turn, put a +1/+1 counter on this creature.",
    )
    .condition(history::descended_this_turn)],
    ..oracle::DEEP_GOBLIN_SKULLTAKER
};
