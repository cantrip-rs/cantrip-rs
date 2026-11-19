use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The Role under the controller, attached to the target; nothing with no target or one no longer legal (CR 608.2b).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::YOUNG_HERO_ROLE, item.controller, creature)?;
    Ok(())
}

/// "When this creature dies, create a Young Hero Role token attached to up to one target creature you control" (CR 111.10r,
/// 303.7): with no target, or one no longer legal as the ability resolves, nothing is created (CR 608.2b). A Role the
/// controller already has on it goes to the graveyard (CR 704.5z).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_DIES,
        create_role,
        "When this creature dies, create a Young Hero Role token attached to up to one target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL.optional()])],
    ..oracle::PROTECTIVE_PARENTS
};

pub static EXTRA: &[&CardDef] = &[&tokens::YOUNG_HERO_ROLE];
