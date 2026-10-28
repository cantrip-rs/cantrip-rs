use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target attacking creature gains first strike until end of turn" (CR 611.2a, 702.7): the creature chosen as the ability went on
/// the stack, if it is still a legal target (CR 608.2b): still attacking.
fn gains_first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    Ok(())
}

/// "Whenever Nori attacks": Nori herself is an attacking creature, so she may be the target.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        gains_first_strike,
        "Whenever Nori attacks, target attacking creature gains first strike until end of turn.",
    )
    .targets(&[targets::ATTACKING_CREATURE])],
    ..oracle::NORI_TELLER_OF_TALES
};
