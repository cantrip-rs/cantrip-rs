use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +1/+3 and gains flying until end of turn. Scry 1." The pump is one sentence's continuous effects
/// (CR 611.2a); a spell whose only target has become illegal does nothing, the scry included (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 3);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
    });
    g.end_sentence();
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::HAMATO_GUARDIAN_STANCE };
