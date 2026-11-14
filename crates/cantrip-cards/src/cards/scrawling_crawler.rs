use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::{AN_OPPONENT_DRAWS_A_CARD, YOUR_UPKEEP};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOUR_UPKEEP, effects::each_player_draws_a_card, "At the beginning of your upkeep, each player draws a card."),
        triggered(AN_OPPONENT_DRAWS_A_CARD, effects::that_player_loses_life::<1>, "Whenever an opponent draws a card, that player loses 1 life."),
    ],
    ..oracle::SCRAWLING_CRAWLER
};
