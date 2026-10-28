use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Remove all counters from target permanent": every kind it has, all of each (CR 122.1, 122.8), from a permanent that is
/// still a legal target as the ability resolves (CR 608.2b). Any permanent may be targeted, with counters or without (the
/// ruling).
fn remove_all_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    effects::remove_all_counters(g, permanent);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(SACRIFICE_THIS, remove_all_counters, "Sacrifice this creature: Remove all counters from target permanent.")
        .targets(&[targets::PERMANENT])],
    ..oracle::VAMPIRE_HEXMAGE
};
