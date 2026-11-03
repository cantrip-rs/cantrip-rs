use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// The target gets +4/-4 if its toughness is at least 5, otherwise +4/-(toughness minus 1).
/// The adjustment is fixed as the spell resolves (CR 608.2h, 611.2a).
fn blood_lust(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let toughness = g.chars(creature).toughness.unwrap_or(0);
    let adjustment = 1i32.saturating_sub(toughness).max(-4);
    pump(g, item.object, creature, 4, adjustment);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], blood_lust),
    ..oracle::BLOOD_LUST
};
