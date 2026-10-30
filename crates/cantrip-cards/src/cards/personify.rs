use cantrip_api::*;
use cantrip_mechanics::tokens::{self, SHAPESHIFTER};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Exile target creature you control, then return that card to the battlefield under its owner's control. Create a 1/1
/// colorless Shapeshifter creature token with changeling." The creature returns as a new object (CR 400.7); a token
/// exiled this way ceases to exist and does not return (CR 111.7). The spell has one target, so if it is illegal on
/// resolution the whole spell does nothing (CR 608.2b) and no token is made.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    cantrip_mechanics::exile_and_return(g, &[creature])?;
    tokens::create(g, item.controller, &SHAPESHIFTER, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL], effect), ..oracle::PERSONIFY };

pub static EXTRA: &[&CardDef] = &[&SHAPESHIFTER];
