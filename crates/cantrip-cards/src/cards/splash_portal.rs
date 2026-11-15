use cantrip_api::*;
use cantrip_mechanics::{effects, exile_and_return_objects, targets};
use cantrip_oracle as oracle;

/// "Exile target creature you control, then return it to the battlefield under its owner's control. If that creature is a Bird,
/// Frog, Otter, or Rat, draw a card." It returns as a new object (CR 400.7), read as it is now, through Splash Portal's own words for
/// the creature types (CR 612); a token exiled this way ceases to exist (CR 111.7) and is nothing to draw for. Nothing happens if the
/// creature is no longer a legal target (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let returned = exile_and_return_objects(g, &[creature])?;
    g.end_sentence();
    let words = g.words(item.object);
    let kinds = [words.subtype(Subtype::Bird), words.subtype(Subtype::Frog), words.subtype(Subtype::Otter), words.subtype(Subtype::Rat)];
    if returned.into_iter().any(|card| g.is_on_battlefield(card) && kinds.iter().any(|&kind| g.chars(card).has_subtype(kind))) {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL], effect), ..oracle::SPLASH_PORTAL };
