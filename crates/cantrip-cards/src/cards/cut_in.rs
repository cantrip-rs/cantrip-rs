use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Cut In deals 4 damage to target creature. Create a Young Hero Role token attached to up to one target creature you control."
/// (CR 120.3, 111.10r, 303.7): the two targets are independent, so the Role is made even when the first target has become illegal,
/// as long as the second is still legal (CR 608.2b); with no second target chosen, or it gone, only the damage is dealt.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_first_target::<4>(g, item)?;
    let Some(creature) = targets::object(g, item, 1) else { return Ok(()) };
    tokens::create_role(g, &tokens::YOUNG_HERO_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE, targets::CREATURE_YOU_CONTROL.optional()], effect),
    ..oracle::CUT_IN
};

pub static EXTRA: &[&CardDef] = &[&tokens::YOUNG_HERO_ROLE];
