use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+0 until end of turn. If that creature is a token, it also gains deathtouch until end of turn"
/// (CR 611.2a, 702.2, 608.2b, 608.2c): nothing happens if the target is no longer legal. The pump is one sentence, the keyword the
/// next, given only to a creature that is a token as the ability resolves.
fn target_gets_plus_one_zero_and_a_token_also_gains_deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, 1, 0);
    g.end_sentence();
    if g.obj(creature).token {
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Deathtouch);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        target_gets_plus_one_zero_and_a_token_also_gains_deathtouch,
        "Whenever you attack, target creature you control gets +1/+0 until end of turn. If that creature is a token, it also gains deathtouch until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::HAZARDROOT_HERBALIST
};
