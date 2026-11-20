use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// The Role under the controller, attached to the target; nothing when the target is no longer legal (CR 608.2b).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::YOUNG_HERO_ROLE, item.controller, creature)?;
    Ok(())
}

/// "{1}, Sacrifice this creature: Create a Young Hero Role token attached to another target creature." (CR 111.10r, 303.7): the
/// target is chosen before the sacrifice is paid (CR 602.2b, 601.2c, 601.2h), while the Veteran is still on the battlefield, and
/// "another" keeps it from being its own target; the Role is made as the ability resolves if its target is still a legal one
/// (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(SACRIFICE_THIS),
        create_role,
        "{1}, Sacrifice this creature: Create a Young Hero Role token attached to another target creature.",
    )
    .targets(&[targets::OTHER_CREATURE])],
    ..oracle::EMBERETH_VETERAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::YOUNG_HERO_ROLE];
