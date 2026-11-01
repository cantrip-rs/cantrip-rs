use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "If another creature entered the battlefield under your control this turn" (CR 603.4): an intervening "if", asked as the end
/// step begins and again as the ability resolves, of the history's `Event::Entered` (CR 603.6a). A creature entering during the
/// end step, after the ability would have triggered, therefore does not make it trigger.
fn another_creature_entered(q: &dyn Query, asker: &Asker) -> bool {
    history::another_creature_entered_under_control_this_turn(q.history(), asker.controller, asker.source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        |g, item| effects::surveil(g, item, 1),
        "At the beginning of your end step, if another creature entered the battlefield under your control this turn, surveil 1.",
    )
    .condition(another_creature_entered)],
    ..oracle::WARY_FARMER
};
