use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::AN_OPPONENT_DRAWS_A_CARD;
use cantrip_oracle as oracle;

/// "during your turn" (CR 102.1, 603.4): a trigger condition (not an intervening "if"): it is checked as the event happens, so a
/// card drawn on an opponent's turn triggers nothing.
fn during_your_turn(q: &dyn Query, me: &Asker) -> bool {
    q.state().turn.active == me.controller
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        AN_OPPONENT_DRAWS_A_CARD,
        tokens::create_a_faerie,
        "Whenever an opponent draws a card during your turn, create a 1/1 blue Faerie creature token with flying.",
    )
    .condition(during_your_turn)],
    ..oracle::TROI_WINGING_IT
};

pub static EXTRA: &[&CardDef] = &[&tokens::FAERIE];
