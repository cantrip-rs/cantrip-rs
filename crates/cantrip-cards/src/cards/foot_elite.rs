use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Another target creature you control gets +1/+0 and gains indestructible until end of turn" (CR 611.2a): both are continuous
/// effects of the resolving ability on the target, one instruction; nothing if the target has left the battlefield (CR 400.7).
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 0);
        effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        rally,
        "Whenever this creature attacks, another target creature you control gets +1/+0 and gains indestructible until end of turn.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::FOOT_ELITE
};
