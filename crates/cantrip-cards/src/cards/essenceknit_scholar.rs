use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{effects, history, tokens};
use cantrip_oracle as oracle;

/// "If a creature died under your control this turn" (CR 603.4, 700.4): asked of the history as the step begins and again as it
/// resolves. Tokens count, and so does the Scholar itself.
fn a_creature_died_under_your_control_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    history::creatures_died_this_turn(q, Some(asker.controller)) > 0
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            tokens::create_a_pest_that_gains_life_on_attack,
            "When this creature enters, create a 1/1 black and green Pest creature token with \"Whenever this token attacks, you gain 1 life.\"",
        ),
        triggered(YOUR_END_STEP, |g, item| effects::draw(g, item, 1), "At the beginning of your end step, if a creature died under your control this turn, draw a card.")
            .condition(a_creature_died_under_your_control_this_turn),
    ],
    ..oracle::ESSENCEKNIT_SCHOLAR
};

pub static EXTRA: &[&CardDef] = &[&tokens::PEST_THAT_GAINS_LIFE_ON_ATTACK];
