use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

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
    mode(effects::target_player_draws_and_loses_life::<2, 2>, "Target player draws two cards and loses 2 life.").targets(&[targets::PLAYER]),
    mode(pump_and_lifelink, "Target creature gets +2/+2 and gains lifelink until end of turn.").targets(&[targets::CREATURE]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::REVERENT_HOWL };
