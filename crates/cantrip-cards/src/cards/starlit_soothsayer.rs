use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::history::you_gained_or_lost_life_this_turn;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "At the beginning of your end step, if you gained or lost life this turn, surveil 1." is an intervening "if" (CR 603.4): checked as
/// the step begins and again as the ability resolves, so it does not trigger at all if no life changed by then.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(YOUR_END_STEP, |g, item| effects::surveil(g, item, 1), "At the beginning of your end step, if you gained or lost life this turn, surveil 1.")
        .condition(you_gained_or_lost_life_this_turn)],
    ..oracle::STARLIT_SOOTHSAYER
};
