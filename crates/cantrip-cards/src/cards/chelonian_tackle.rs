use cantrip_api::*;
use cantrip_mechanics::{fight, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +0/+10 until end of turn. Then it fights up to one target creature an opponent
/// controls." With the first target illegal nothing happens; with no second target, or an illegal one, only the pump does
/// (CR 701.14b).
fn tackle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, 0, 10);
    g.end_sentence();
    fight::targets_fight(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS.optional()], tackle),
    ..oracle::CHELONIAN_TACKLE
};
