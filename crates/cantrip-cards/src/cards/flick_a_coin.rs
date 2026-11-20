use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Flick a Coin deals 1 damage to any target. You create a Treasure token. Draw a card." (CR 120.3, 111.10a, 121.1): with its only
/// target gone as it resolves the whole spell does nothing (CR 608.2b), so no Treasure is made and no card is drawn.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_first_target::<1>(g, item)?;
    tokens::create_treasure(g, item.controller)?;
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ANY], effect), ..oracle::FLICK_A_COIN };

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
