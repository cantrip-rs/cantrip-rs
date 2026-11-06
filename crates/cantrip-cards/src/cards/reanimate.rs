use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Put target creature card from a graveyard onto the battlefield under your control; you lose life equal to its
/// mana value.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let value = g.mana_value(card) as i32;
    g.put_onto_battlefield(card, item.controller, false)?;
    g.lose_life(item.controller,(value) as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_A_GRAVEYARD], effect),
    ..oracle::REANIMATE
};
