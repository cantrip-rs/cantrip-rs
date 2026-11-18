use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn. Suspect it."
/// (CR 701.60a): the first three sentences are Act of Treason's, and the fourth makes the creature suspected, which stays
/// (CR 701.60c: until it leaves the battlefield) after control returns at the end of turn. A target that is no longer
/// legal does nothing (CR 608.2b). "This spell can't be countered" is the printed keyword.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = effects::gain_control_untap_haste(g, item) else { return Ok(()) };
    g.end_sentence();
    g.suspect(creature);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::CAUGHT_RED_HANDED };
