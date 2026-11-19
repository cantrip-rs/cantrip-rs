use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target creature or enchantment. Create a Wicked Role token attached to up to one target creature you control." (CR 701.8,
/// 111.10, 608.2b-c): each target is rechecked as it resolves and the instruction that needs one that is gone is skipped, the other
/// still done; the destruction is a sentence of its own, so a creature of yours it destroys is already in the graveyard when the Role is
/// made, and (the same creature being both targets) gets none (CR 400.7).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(target) = targets::object(g, item, 0) {
        g.destroy(target);
    }
    g.end_sentence();
    let Some(creature) = targets::object(g, item, 1) else { return Ok(()) };
    tokens::create_role(g, &tokens::WICKED_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(targets::creature_or_enchantment), targets::CREATURE_YOU_CONTROL.optional()], effect),
    ..oracle::SHATTER_THE_OATH
};

pub static EXTRA: &[&CardDef] = &[&tokens::WICKED_ROLE];
