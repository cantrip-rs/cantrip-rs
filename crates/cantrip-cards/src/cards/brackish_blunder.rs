use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Return target creature to its owner's hand. If it was tapped, create a Map token" (CR 400.3, 608.2c, 111.10s): whether the creature
/// was tapped is read as the spell resolves, before it leaves (CR 608.2h), and the Map is made whether or not it really arrived in a
/// hand. A spell whose only target has become illegal does nothing at all (CR 608.2b).
fn return_then_map_if_it_was_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let tapped = g.obj(creature).tapped;
    g.return_to_hand(&[creature]);
    g.end_sentence();
    if tapped {
        tokens::create_map(g, item.controller)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], return_then_map_if_it_was_tapped),
    ..oracle::BRACKISH_BLUNDER
};
