use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return target nonland permanent to its owner's hand. Surveil 1." A spell whose only target has become illegal does
/// nothing at all (CR 608.2b), so there is no surveil then; otherwise the surveil follows whether or not the permanent
/// actually moved.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.return_to_hand(&[permanent]);
    g.end_sentence();
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::NONLAND_PERMANENT], effect), ..oracle::BANISHING_BETRAYAL };
