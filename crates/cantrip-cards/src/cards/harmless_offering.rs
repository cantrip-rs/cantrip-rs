use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target opponent gains control of target permanent you control." (CR 108.4)
fn donate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_opponent_gains_control_of_target_permanent(g, item);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::OPPONENT.labelled("Opponent who gains control of the permanent"),
            targets::PERMANENT_YOU_CONTROL.labelled("Permanent you control for that opponent to gain control of"),
        ],
        donate,
    ),
    ..oracle::HARMLESS_OFFERING
};
