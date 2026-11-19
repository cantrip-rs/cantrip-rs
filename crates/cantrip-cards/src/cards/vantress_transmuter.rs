use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Tap target creature. Create a Cursed Role token attached to it." (CR 701.26a, 303.7, 608.2b): the Role goes on the creature
/// whether or not it was already tapped, and neither happens if the target is no longer legal.
fn tap_and_curse(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_targets(g, item)?;
    g.end_sentence();
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::CURSED_ROLE, item.controller, creature)?;
    Ok(())
}

/// The Adventure (CR 715): the Role is the spell's controller's, whoever controls the creature.
pub static CROAKING_CURSE: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], tap_and_curse),
    ..oracle::CROAKING_CURSE
};

pub static CARD: CardDef = CardDef { adventure: Some(&CROAKING_CURSE), ..oracle::VANTRESS_TRANSMUTER };

pub static EXTRA: &[&CardDef] = &[&CROAKING_CURSE, &tokens::CURSED_ROLE];
