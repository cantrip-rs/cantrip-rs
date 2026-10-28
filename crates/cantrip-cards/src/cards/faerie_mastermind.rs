use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::AN_OPPONENT_DRAWS_THEIR_SECOND_CARD, |g, item| cantrip_mechanics::effects::draw(g, item, 1), "Whenever an opponent draws their second card each turn, you draw a card."),
        ability(cost("{3}{U}"), cantrip_mechanics::effects::each_player_draws_a_card, "{3}{U}: Each player draws a card."),
    ],
    ..oracle::FAERIE_MASTERMIND
};
