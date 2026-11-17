use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, history, solved, triggers::INSTANT_OR_SORCERY_YOU_CAST};
use cantrip_oracle as oracle;

/// "To solve — You've cast four or more instant and sorcery spells this turn" (CR 719.3a, 601.2i): an intervening "if" (CR 603.4), asked as
/// the end step begins and as the ability resolves; the spells the Case's controller cast this turn count, those cast before the Case
/// arrived and countered ones included, an opponent's do not.
fn cast_four_instants_and_sorceries_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    history::instants_and_sorceries_cast_this_turn(q, asker.controller) >= 4
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        solved::to_solve(
            cast_four_instants_and_sorceries_this_turn,
            "To solve \u{2014} You've cast four or more instant and sorcery spells this turn. (If unsolved, solve at the beginning of your end step.)",
        ),
        solved::triggered(INSTANT_OR_SORCERY_YOU_CAST, |g, item| effects::draw(g, item, 1), "Solved \u{2014} Whenever you cast an instant or sorcery spell, draw a card."),
    ],
    // "Instant and sorcery spells you cast cost {1} less to cast" (CR 601.2f, 118.7), solved or not.
    cost_changes: &[cost_reduction::instants_and_sorceries_you_cast_cost_less::<1>],
    ..oracle::CASE_OF_THE_RANSACKED_LAB
};
