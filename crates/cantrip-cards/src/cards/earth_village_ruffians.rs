use cantrip_api::*;
use cantrip_mechanics::{earthbend, triggers::THIS_DIES};
use cantrip_oracle as oracle;

/// "When this creature dies, earthbend 2" (CR 701.66a): the shared earthbend effect, its land chosen as the trigger goes on the
/// stack, and `RETURN` its delayed return.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, earthbend::effect::<2>, "When this creature dies, earthbend 2. (Target land you control becomes a 0/0 creature with haste that's still a land. Put two +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped.)")
        .targets(&[earthbend::TARGET])],
    delayed: &[earthbend::RETURN],
    ..oracle::EARTH_VILLAGE_RUFFIANS
};
