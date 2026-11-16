use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Return target nonland permanent to its owner's hand. If you control a Desert, surveil 1." The Desert is looked for as the spell
/// resolves, after the permanent has moved (CR 608.2c). A spell whose only target is illegal does not resolve at all (CR 608.2b),
/// so it never surveils.
fn return_then_surveil_with_a_desert(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_hand(g, item)?;
    g.end_sentence();
    let desert = g.words(item.object).subtype(Subtype::Desert);
    if g.permanents_of(item.controller).into_iter().any(|id| g.chars(id).has_subtype(desert)) {
        effects::surveil(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::NONLAND_PERMANENT], return_then_surveil_with_a_desert), ..oracle::FAILED_FORDING };
