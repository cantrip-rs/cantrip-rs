use cantrip_api::*;
use cantrip_mechanics::tokens::MUTAGEN;
use cantrip_mechanics::{counter_target, targets, tokens};
use cantrip_oracle as oracle;

/// "Counter target spell. Create a Mutagen token." With its one target gone or illegal the spell does not resolve at all
/// (CR 608.2b: the core removes it from the stack), so no token is made.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_target(g, item, 0);
    g.end_sentence();
    tokens::create(g, item.controller, &MUTAGEN, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effect),
    ..oracle::OOZE_SPILL
};

/// The Mutagen token it makes.
pub static EXTRA: &[&CardDef] = &[&MUTAGEN];
