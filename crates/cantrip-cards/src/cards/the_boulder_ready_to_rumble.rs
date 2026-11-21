use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{earthbend, ferocious};
use cantrip_oracle as oracle;

/// "Earthbend X, where X is the number of creatures you control with power 4 or greater" (CR 701.66a, 107.3): counted as the
/// ability resolves (CR 608.2h), before the land becomes a creature (`ferocious::count`). Only a creature has a power (CR 208.3), so a
/// noncreature permanent with the same number printed is never counted.
fn earthbend_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = ferocious::count(g, item.controller) as i32;
    earthbend::earthbend(g, item, x)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        earthbend_x,
        "Whenever The Boulder attacks, earthbend X, where X is the number of creatures you control with power 4 or greater. (Target land you control becomes a 0/0 creature with haste that's still a land. Put X +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped.)",
    )
    .targets(&[earthbend::TARGET])],
    delayed: &[earthbend::RETURN],
    ..oracle::THE_BOULDER_READY_TO_RUMBLE
};
