use cantrip_api::*;
use cantrip_mechanics::{pump, targets, trample_until_end_of_turn, triggers};
use cantrip_oracle as oracle;

/// "Another target creature you control gains trample and gets +X/+X until end of turn, where X is this creature's power"
/// (CR 611.2a, 608.2h): X is worked out once, as the ability resolves, from the Animist's power then, or as it last was on the
/// battlefield if it has left (CR 113.7a); a negative power gives zero (CR 107.1b). Nothing happens if the target is no longer legal
/// (CR 608.2b).
fn target_gains_trample_and_gets_plus_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = targets::power_or_last_known(g, item.object).max(0);
    // One instruction (`Game::one_instruction`): "gains trample and gets +X/+X" is one sentence joined by "and".
    g.one_instruction(&mut |g| {
        trample_until_end_of_turn(g, item.object, creature);
        pump(g, item.object, creature, x, x);
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        target_gains_trample_and_gets_plus_its_power,
        "Whenever this creature attacks, another target creature you control gains trample and gets +X/+X until end of turn, where X is this creature's power.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::ASHROOT_ANIMIST
};
