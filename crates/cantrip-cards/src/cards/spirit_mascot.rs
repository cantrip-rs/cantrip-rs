use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more cards leave your graveyard, put a +1/+1 counter on this creature": one trigger for the cards that left
/// together (CR 603.2c). Nothing if it has left the battlefield by the time the ability resolves (CR 400.7).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CARDS_LEAVE_YOUR_GRAVEYARD,
        effects::plus_one_counter_on_this,
        "Whenever one or more cards leave your graveyard, put a +1/+1 counter on this creature.",
    )
    .once_per_batch()],
    ..oracle::SPIRIT_MASCOT
};
