use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +1/+3 and gains flying until end of turn. Untap it." (CR 611.2a, 701.26b): the pump and the flying are
/// one sentence's continuous effects, then the creature untaps; a spell whose only target has become illegal does nothing, the
/// untap included (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::target_creature_gets_and_gains_flying::<1, 3>(g, item)?;
    g.end_sentence();
    g.untap(creature);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::ACROBATIC_LEAP };
