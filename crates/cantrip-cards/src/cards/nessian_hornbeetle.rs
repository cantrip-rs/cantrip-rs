use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

/// "If you control another creature with power 4 or greater", asked of the ability's controller and source.
fn another_with_power_4_or_greater(q: &dyn Query, asker: &Asker) -> bool {
    ferocious::you_control_another(q, asker.controller, asker.source)
}

/// "At the beginning of combat on your turn, if you control another creature with power 4 or greater, put a +1/+1 counter on this
/// creature." An intervening "if" (CR 603.4): asked as combat begins and again as the ability resolves, so it need not be the
/// same creature at both times (the rulings); one counter however many creatures qualify.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        effects::plus_one_counter_on_this,
        "At the beginning of combat on your turn, if you control another creature with power 4 or greater, put a +1/+1 counter on this creature.",
    )
    .condition(another_with_power_4_or_greater)],
    ..oracle::NESSIAN_HORNBEETLE
};
