use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// The Role under the caster, attached to the target; nothing when the target is no longer legal (CR 608.2b).
fn create_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::ROYAL_ROLE, item.controller, creature)?;
    Ok(())
}

/// The Adventure (CR 715): "Create a Royal Role token attached to target creature you control." The Role is attached by the
/// effect, not targeted (the Role rulings); a Role the caster already controls on it goes to its owner's graveyard (CR 704.5z).
pub static BETROTH_THE_BEAST: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], create_role),
    ..oracle::BETROTH_THE_BEAST
};

pub static CARD: CardDef = CardDef { adventure: Some(&BETROTH_THE_BEAST), ..oracle::BESOTTED_KNIGHT };

pub static EXTRA: &[&CardDef] = &[&BETROTH_THE_BEAST, &tokens::ROYAL_ROLE];
