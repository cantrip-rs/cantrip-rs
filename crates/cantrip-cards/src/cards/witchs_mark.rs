use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "You may discard a card. If you do, draw two cards. Create a Wicked Role token attached to up to one target creature you
/// control." (CR 701.9, 303.7, 608.2c): the loot is its own sentence, so the Role's creature is judged after it; the Role is made
/// whether or not a card was discarded.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_discard_to_draw(g, item, 2)?;
    g.end_sentence();
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::WICKED_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL.optional()], effect), ..oracle::WITCHS_MARK };

pub static EXTRA: &[&CardDef] = &[&tokens::WICKED_ROLE];
