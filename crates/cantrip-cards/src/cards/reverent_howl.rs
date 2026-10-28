use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Mode 1: "Target player draws two cards and loses 2 life." (CR 121.1, 119.3): nothing if the player is no longer a legal
/// target (CR 608.2b).
fn draw_and_lose(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    g.draw(player, 2);
    g.lose_life(player, 2);
    Ok(())
}

/// Mode 2: "Target creature gets +2/+2 and gains lifelink until end of turn." (CR 611.2a, 702.15): one instruction.
fn pump_and_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        cantrip_mechanics::pump(g, item.object, creature, 2, 2);
        effects::gain_lifelink_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(draw_and_lose, "Target player draws two cards and loses 2 life.").targets(&[targets::PLAYER]),
    mode(pump_and_lifelink, "Target creature gets +2/+2 and gains lifelink until end of turn.").targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::REVERENT_HOWL };
