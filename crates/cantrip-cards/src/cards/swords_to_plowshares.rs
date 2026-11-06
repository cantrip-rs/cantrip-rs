use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::targets;

/// Exile target creature; its controller gains life equal to its power, as it last existed on the battlefield
/// (CR 608.2h).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let power = g.chars(creature).power.unwrap_or(0);
    let controller = g.controller(creature);
    g.exile(creature);
    if power > 0 {
        g.gain_life(controller,(power as i32) as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::SWORDS_TO_PLOWSHARES
};
