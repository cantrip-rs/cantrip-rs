use cantrip_api::*;
use cantrip_mechanics::tokens::{self, TREASURE};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Counter target noncreature spell. Its controller (the one it had on the stack, CR 608.2h) creates two Treasure
/// tokens, whether or not it could be countered; if the target is no longer legal, nothing happens (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(spell);
    effects::counter_first_target(g, item)?;
    tokens::create(g, controller, &TREASURE, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::NONCREATURE_SPELL], effect),
    ..oracle::AN_OFFER_YOU_CANT_REFUSE
};

pub static EXTRA: &[&CardDef] = &[&TREASURE];
