use cantrip_api::*;
use cantrip_mechanics::{card_types_in_graveyard, delirium, effects, triggers};
use cantrip_oracle as oracle;

/// "Delirium — As long as there are four or more card types among cards in your graveyard, each opponent's maximum hand
/// size ...": the rule applies to an opponent of Winter's controller while that controller has delirium.
fn opponent_while_delirious(q: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player != asker.controller && delirium(q, asker.controller)
}

/// "... is equal to seven minus the number of those card types": seven less the types among the cards in the controller's
/// graveyard (never fewer than no cards, CR 107.1b).
fn seven_minus_card_types_in_your_graveyard(q: &dyn Query, asker: &Asker) -> Option<u32> {
    Some(7u32.saturating_sub(card_types_in_graveyard(q, asker.controller) as u32))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2}"),
        triggered(triggers::YOUR_UPKEEP, effects::each_player_draws::<2>, "At the beginning of your upkeep, each player draws two cards."),
    ],
    play_rules: &[PlayRule::MaximumHandSizeOf(opponent_while_delirious, seven_minus_card_types_in_your_graveyard)],
    ..oracle::WINTER_MISANTHROPIC_GUIDE
};
