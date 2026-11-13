use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

/// "Max speed — At the beginning of your upkeep, exile the top card of your library. You may play that card this turn"
/// (CR 702.178a): the triggered ability the creature is granted while its controller's speed is 4, the first of the card's
/// `delayed`.
pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    delayed: &[triggered(
        triggers::YOUR_UPKEEP,
        library::exile_top_card_play_this_turn,
        "At the beginning of your upkeep, exile the top card of your library. You may play that card this turn.",
    )],
    statics: &[max_speed("Max speed \u{2014} At the beginning of your upkeep, exile the top card of your library. You may play that card this turn.")],
    ..oracle::ENDRIDER_SPIKESPITTER
};
