use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Creatures you control get +2/+1 until end of turn. Investigate." (CR 611.2c, 701.16a): the pump first, for the creatures
/// that are on the battlefield as the spell resolves (a creature entering later gets nothing), then, as a second instruction,
/// the Clue.
fn on_the_job(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control(g, item, 2, 1);
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], on_the_job), ..oracle::ON_THE_JOB };
pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
