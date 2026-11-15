use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::history::another_creature_entered_under_your_control_this_turn;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "If another creature entered the battlefield under your control this turn" (CR 603.4): an intervening "if", asked as the end
/// step begins and again as the ability resolves, of the history's `Event::Entered` (CR 603.6a). A creature entering during the
/// end step, after the ability would have triggered, therefore does not make it trigger.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        |g, item| effects::surveil(g, item, 1),
        "At the beginning of your end step, if another creature entered the battlefield under your control this turn, surveil 1.",
    )
    .condition(another_creature_entered_under_your_control_this_turn)],
    ..oracle::WARY_FARMER
};
