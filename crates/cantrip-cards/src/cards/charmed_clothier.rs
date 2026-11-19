use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The Role under the controller, attached to the target; nothing when the target is no longer legal (CR 608.2b).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::ROYAL_ROLE, item.controller, creature)?;
    Ok(())
}

/// "When this creature enters, create a Royal Role token attached to another target creature you control." The Role is created
/// and attached by the effect, not cast, so the creature's hexproof or protection does not stop it, and with the target illegal
/// on resolution nothing happens (CR 608.2b, 303.4f). The Clothier itself is no legal target (CR 115.1).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_role,
        "When this creature enters, create a Royal Role token attached to another target creature you control.",
    )
    .targets(&[targets::OTHER_CREATURE_YOU_CONTROL])],
    ..oracle::CHARMED_CLOTHIER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ROYAL_ROLE];
