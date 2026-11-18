use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, effects, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature gets +1/+1 for each creature you control and gains reach." One instruction: the number of
/// creatures is counted once, as the spell resolves (CR 608.2h, ruling 2024-02-02), the target itself and a target an opponent controls
/// included; later creatures change nothing. Nothing happens if the target is no longer legal (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let count = creatures_you_control(g, item.controller) as i32;
    instruction(g, |g| {
        cantrip_mechanics::pump(g, item.object, creature, count, count);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Reach);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::GET_A_LEG_UP
};
