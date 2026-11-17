use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Exile target creature; if it had mana value 3 or less, surveil 2. The mana value is read before the exile, while it is still on the battlefield
/// (X in its cost counts as 0 there, CR 202.3e). A target that is illegal on
/// resolution means the spell does nothing at all (CR 608.2b).
fn exile_then_surveil(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let value = g.mana_value(creature);
    g.exile(creature);
    g.end_sentence();
    if value <= 3 {
        effects::surveil(g, item, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], exile_then_surveil),
    ..oracle::CONSUMING_ASHES
};
