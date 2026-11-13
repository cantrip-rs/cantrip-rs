use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Flash is a core keyword. "Whenever you cast a spell during an opponent's turn, put a +1/+1 counter on this creature" (CR 603.2,
/// 122.1b): it is on the battlefield by then, so casting it (with flash) on an opponent's turn gives it no counter.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::SPELL_YOU_CAST_DURING_AN_OPPONENTS_TURN,
        effects::plus_one_counter_on_this,
        "Whenever you cast a spell during an opponent's turn, put a +1/+1 counter on this creature.",
    )],
    ..oracle::BRINEBORN_CUTTHROAT
};
