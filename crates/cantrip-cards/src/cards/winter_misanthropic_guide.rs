use cantrip_api::*;
use cantrip_mechanics::{card_types_in_graveyard, delirium, effects, triggers};
use cantrip_oracle as oracle;

/// "Delirium — As long as there are four or more card types among cards in your graveyard, each opponent's maximum hand
/// size is equal to seven minus the number of those card types": the rule applies to an opponent of Winter's controller while
/// that controller has delirium, and says seven less the types among the cards in the controller's graveyard (never fewer than
/// no cards, CR 107.1b).
fn opponents_maximum_hand_size(q: &dyn Query, asker: &Asker, player: PlayerId) -> Option<u32> {
    if player == asker.controller || !delirium(q, asker.controller) {
        return None;
    }
    Some(7u32.saturating_sub(card_types_in_graveyard(q, asker.controller) as u32))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2}"),
        triggered(triggers::YOUR_UPKEEP, effects::each_player_draws::<2>, "At the beginning of your upkeep, each player draws two cards."),
    ],
    play_rules: &[PlayRule::MaximumHandSizeOf(opponents_maximum_hand_size)],
    ..oracle::WINTER_MISANTHROPIC_GUIDE
};
