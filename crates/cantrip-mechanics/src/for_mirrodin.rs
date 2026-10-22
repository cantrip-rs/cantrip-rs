//! For Mirrodin! (CR 702.161a): "When this Equipment enters, create a 2/2 red Rebel creature token, then attach this to
//! it." Glimmer Lens prints it.
//!
//! A card with For Mirrodin! lists `TRIGGER` among its `abilities` and `REBEL` among its `EXTRA` definitions.

use crate::triggers::THIS_ENTERS;
use cantrip_api::*;

/// The 2/2 red Rebel creature token For Mirrodin! makes.
pub static REBEL: CardDef = CardDef {
    name: "Rebel Token",
    token_name: Some(&[Subtype::Rebel]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Rebel]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// An Equipment that has left the battlefield attaches to nothing, and a token that was not created is not there to be
/// equipped.
fn create_and_attach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rebel = g.card_id(&REBEL);
    // The Rebel is the keyword's (CR 702.163a), whatever the card's text says now.
    let made = g.create_tokens_worded(&[(rebel, item.controller)], false, None, Words::NONE)?;
    let Some(&token) = made.first() else { return Ok(()) };
    if is_live(g, token) && g.is_on_battlefield(token) && is_live(g, item.object) && g.is_on_battlefield(item.object) {
        g.attach(item.object, token);
    }
    Ok(())
}

pub const TRIGGER: AbilityDef = triggered(THIS_ENTERS, create_and_attach, "For Mirrodin! (When this Equipment enters, create a 2/2 red Rebel creature token, then attach this to it.)");
