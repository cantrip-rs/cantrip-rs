use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "You may have it fight target creature you don't control" (CR 701.14a-b, 608.2b): the target is chosen as the ability goes on
/// the stack and the fight chosen as it resolves; with the target illegal by then the ability does not resolve, and with the Indrik
/// gone neither creature fights (CR 701.14b).
fn may_fight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(prey) = targets::object(g, item, 0) else { return Ok(()) };
    fight::may_have_this_fight(g, item, prey)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, may_fight, "When this creature enters, you may have it fight target creature you don't control.")
        .targets(&[targets::CREATURE_YOU_DONT_CONTROL])],
    ..oracle::AFFECTIONATE_INDRIK
};
