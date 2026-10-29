use cantrip_api::*;
use cantrip_mechanics::tokens::MUTAGEN;
use cantrip_mechanics::{library, targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature's owner puts it on their choice of the top or bottom of their library. You create a Mutagen token."
/// With its one target gone or illegal the spell does not resolve at all
/// (CR 608.2b: the core removes it from the stack), so no token is made.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        library::owner_puts_on_top_or_bottom(g, item.object, creature)?;
    }
    g.end_sentence();
    tokens::create(g, item.controller, &MUTAGEN, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::RETURN_TO_THE_SEWERS
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
