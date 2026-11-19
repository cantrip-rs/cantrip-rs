use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Create a Cursed Role token attached to target creature you control" (CR 111.10j, 303.7): the creature is chosen as the
/// ability is put on the stack (it may be this creature) and the Role is created attached to it if it is still a legal target;
/// the creature's own controller keeps only the newest Role of theirs on it (CR 704.5z).
fn cursed_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::CURSED_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        cursed_role,
        "When this creature enters, create a Cursed Role token attached to target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::SPITEFUL_HEXMAGE
};

pub static EXTRA: &[&CardDef] = &[&tokens::CURSED_ROLE];
