use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +X/+0 and gains first strike until end of turn." (CR 107.3a, 611.2a, 702.7a): one instruction, X being the
/// value paid in the cost; nothing if the target is no longer legal as it resolves (CR 608.2b).
fn gets_x_and_first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, item.x as i32, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], gets_x_and_first_strike), ..oracle::PEDAL_TO_THE_METAL };
