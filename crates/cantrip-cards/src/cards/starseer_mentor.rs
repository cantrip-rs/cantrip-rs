use cantrip_api::*;
use cantrip_mechanics::history::you_gained_or_lost_life_this_turn;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The end-step trigger has an intervening "if you gained or lost life this turn" (CR 603.4): checked as the step begins and again as the
/// ability resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        effects::target_player_loses_life_unless_they_sacrifice_a_nonland_permanent_or_discard_a_card::<3>,
        "At the beginning of your end step, if you gained or lost life this turn, target opponent loses 3 life unless they sacrifice a nonland permanent of their choice or discard a card.",
    )
    .targets(&[targets::OPPONENT])
    .condition(you_gained_or_lost_life_this_turn)],
    labels: &["Sacrifice a nonland permanent", "Discard a card", "Lose 3 life"],
    ..oracle::STARSEER_MENTOR
};
