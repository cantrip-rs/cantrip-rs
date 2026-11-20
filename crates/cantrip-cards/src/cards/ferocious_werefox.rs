use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// The Role under the caster, attached to the target; nothing when the target is no longer legal (CR 608.2b).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::MONSTER_ROLE, item.controller, creature)?;
    Ok(())
}

/// The Adventure (CR 715.3b, 715.3d): Guard Change, "Create a Monster Role token attached to target creature you control."
pub static GUARD_CHANGE: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL], create_role), ..oracle::GUARD_CHANGE };

pub static CARD: CardDef = CardDef { adventure: Some(&GUARD_CHANGE), ..oracle::FEROCIOUS_WEREFOX };

pub static EXTRA: &[&CardDef] = &[&GUARD_CHANGE, &tokens::MONSTER_ROLE];
