use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast an instant or sorcery spell, create a Sorcerer Role token attached to up to one other target creature you
/// control." With no target chosen there is nothing to attach to, so no Role is made (CR 303.4i); if the chosen creature has left or
/// stopped being legal when the trigger resolves it makes none either (CR 608.2b). A Role the caster already controls on that creature
/// goes to the graveyard (CR 704.5z).
fn sorcerer_role_on_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::SORCERER_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        sorcerer_role_on_target,
        "Whenever you cast an instant or sorcery spell, create a Sorcerer Role token attached to up to one other target creature you control.",
    )
    .targets(&[up_to_one(targets::OTHER_CREATURE_YOU_CONTROL.filter)])],
    ..oracle::SPLASHY_SPELLCASTER
};

pub static EXTRA: &[&CardDef] = &[&tokens::SORCERER_ROLE];
