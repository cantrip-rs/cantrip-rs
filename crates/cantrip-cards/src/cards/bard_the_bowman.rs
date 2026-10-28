use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever you draw your second card each turn, put a +1/+1 counter on target creature. It gains lifelink until end of turn" (CR
/// 122.1, 611.2a, 702.15): once each turn, whether or not Bard was on the battlefield for the first draw (the ruling). Any creature
/// may be the target, an opponent's among them.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_DRAW_YOUR_SECOND_CARD,
        plus_one_counters_on_first_target_that_gains_until_end_of_turn::<1, { Keywords::of(&[Keyword::Lifelink]).0 }>,
        "Whenever you draw your second card each turn, put a +1/+1 counter on target creature. It gains lifelink until end of turn.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::BARD_THE_BOWMAN
};
