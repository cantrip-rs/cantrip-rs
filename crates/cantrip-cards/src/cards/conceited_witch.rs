use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// The Wicked Role under the caster, attached to the target (CR 111.10q, 303.7).
fn wicked_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::WICKED_ROLE, item.controller, creature)?;
    Ok(())
}

/// The Adventure (CR 715): "Create a Wicked Role token attached to target creature you control." Nothing is created if the
/// creature is gone as it resolves (CR 608.2b), and the spell is then not exiled on an adventure but goes to the graveyard.
pub static PRICE_OF_BEAUTY: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], wicked_role),
    ..oracle::PRICE_OF_BEAUTY
};

pub static CARD: CardDef = CardDef { adventure: Some(&PRICE_OF_BEAUTY), ..oracle::CONCEITED_WITCH };

pub static EXTRA: &[&CardDef] = &[&PRICE_OF_BEAUTY, &tokens::WICKED_ROLE];
