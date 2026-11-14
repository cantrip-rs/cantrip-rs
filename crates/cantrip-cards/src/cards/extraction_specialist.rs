use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Return target creature card with mana value 2 or less from your graveyard to the battlefield. That creature can't
/// attack or block for as long as you control this creature." The card comes back under the ability's controller (CR
/// 400.3, 110.2a); if this creature has already left (or is not yours), the restriction does nothing (CR 611.2b, the ruling).
fn raise(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let arrived = g.put_onto_battlefield(card, item.controller, false)?;
    effects::cant_attack_or_block_while_you_control_source(g, item, arrived);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        raise,
        "When this creature enters, return target creature card with mana value 2 or less from your graveyard to the battlefield. That creature can't attack or block for as long as you control this creature.",
    )
    .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_2_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::EXTRACTION_SPECIALIST
};
