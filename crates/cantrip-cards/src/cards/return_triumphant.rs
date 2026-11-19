use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Return target creature card with mana value 3 or less from your graveyard to the battlefield. Create a Young Hero Role token
/// attached to it." (CR 400.3, 111.10r, 303.7): the Role goes on the permanent the card became. It is attached by the spell, not
/// targeted, and is not created if that permanent is no longer on the battlefield (CR 303.4i); nothing happens if the target is
/// no longer legal (CR 608.2b).
fn return_and_make_a_young_hero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let arrived = g.put_onto_battlefield(card, item.controller, false)?;
    tokens::create_role(g, &tokens::YOUNG_HERO_ROLE, item.controller, arrived)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD], return_and_make_a_young_hero),
    ..oracle::RETURN_TRIUMPHANT
};

pub static EXTRA: &[&CardDef] = &[&tokens::YOUNG_HERO_ROLE];
