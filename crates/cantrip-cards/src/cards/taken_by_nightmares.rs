use cantrip_api::*;
use cantrip_mechanics::{effects, targets, you_control_an_enchantment};
use cantrip_oracle as oracle;

/// "Exile target creature. If you control an enchantment, scry 2." (CR 701.13a, 701.22, 608.2c): the exile first, then the
/// enchantment check as the second sentence resolves, so a Role the exiled creature wore is already gone and does not count. With
/// the target no longer legal the spell does not resolve at all, so the scry is not done either (CR 608.2b).
fn exile_and_maybe_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::exile_first_target(g, item)?;
    g.end_sentence();
    if you_control_an_enchantment(g, item.controller) {
        effects::scry_two(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], exile_and_maybe_scry), ..oracle::TAKEN_BY_NIGHTMARES };
