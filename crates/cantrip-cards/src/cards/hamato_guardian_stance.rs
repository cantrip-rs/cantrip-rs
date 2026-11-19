use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +1/+3 and gains flying until end of turn. Scry 1." The pump is one sentence's continuous effects
/// (CR 611.2a); a spell whose only target has become illegal does nothing, the scry included (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::object(g, item, 0).is_none() {
        return Ok(());
    }
    effects::target_creature_gets_and_gains_flying::<1, 3>(g, item)?;
    g.end_sentence();
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::HAMATO_GUARDIAN_STANCE };
