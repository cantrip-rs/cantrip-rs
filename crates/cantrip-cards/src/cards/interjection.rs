use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +2/+2 and gains first strike until end of turn" (CR 611.2a): one instruction. Nothing if the target
/// is no longer legal as the spell resolves (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 2, 2);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::INTERJECTION
};
