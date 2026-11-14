use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +0/+10 until end of turn. Then it fights up to one target creature an opponent
/// controls."
fn tackle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::pump_first_target_then_it_fights_second(g, item, 0, 10);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS.optional()], tackle),
    ..oracle::CHELONIAN_TACKLE
};
