use cantrip_api::*;
use cantrip_mechanics::{history, library, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of each player's end step, if an artifact entered the battlefield under your control this turn" (CR 603.4,
/// 603.6a): the turn's history, whoever's end step it is, asked as the step begins and again as the ability resolves. "Look at the
/// top two cards of your library. Put one of them into your hand and the other into your graveyard" (CR 701.20e, 400.3) is for
/// Akal Pakal's controller.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_END_STEP,
        |g, item| library::look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 2),
        "At the beginning of each player's end step, if an artifact entered the battlefield under your control this turn, look at the top two cards of your library. Put one of them into your hand and the other into your graveyard.",
    )
    .condition(|q, asker| history::artifact_entered_under_control_this_turn(q.history(), asker.controller))],
    ..oracle::AKAL_PAKAL_FIRST_AMONG_EQUALS
};
