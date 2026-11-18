//! Ripple N (CR 702.60a): "When you cast this spell, you may reveal the top N cards of your library. You may cast spells
//! with the same name as this spell from among the revealed cards without paying their mana costs. Put the rest on the
//! bottom of your library in any order." Thrumming Stone gives it to the spells its controller casts (4).
//!
//! A card that gives or has ripple lists `trigger::<N>(text)`: it functions from the stack, on the spell cast.

use crate::{choose, same_name};
use cantrip_api::*;

/// Its controller may reveal the top `N` cards of their library (all of them if there are fewer), may cast any number of
/// the revealed cards with the same name as the spell without paying their mana costs, one at a time, then puts every
/// revealed card not cast on the bottom in the order they choose. "The same name" is the spell's as it is now or as it
/// last was (CR 608.2h).
fn ripple<const N: usize>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let spell = item.object;
    let library: Vec<ObjectId> = g.state().zone(Zone::library(you)).iter().take(N).copied().collect();
    if library.is_empty() {
        return Ok(());
    }
    if g.ask(you, RequestKind::May { source: spell, purpose: Purpose::Ask(&REVEAL_FOR_RIPPLE) })? != Answer::Yes {
        return Ok(());
    }
    let revealed = library;
    g.reveal(you, &revealed);

    loop {
        let in_library: Vec<ObjectId> =
            revealed.iter().copied().filter(|&c| is_live(g, c) && g.obj(c).zone.is(ZoneKind::Library)).collect();
        let castable: Vec<ObjectId> =
            in_library.iter().copied().filter(|&c| same_name(g, c, spell) && g.could_cast_during_resolution(c)).collect();
        if castable.is_empty() {
            break;
        }
        let Some(&card) = choose::cards_from(g, you, spell, Purpose::CastFree, in_library, castable, 1, true)?.first() else { break };
        g.cast_during_resolution(card, Some(ManaCost::ZERO))?;
    }

    // What was cast has left the library; the rest go to the bottom in the order chosen.
    let rest: Vec<ObjectId> =
        revealed.iter().copied().filter(|&c| is_live(g, c) && g.obj(c).zone.is(ZoneKind::Library)).collect();
    if rest.is_empty() {
        return Ok(());
    }
    let mut order = rest.clone();
    if rest.len() > 1 {
        let ask = RequestKind::DistributeCards {
            source: spell,
            cards: rest.clone(),
            destinations: vec![Destination { kind: DestinationKind::LibraryBottom, min: 0, max: u32::MAX, eligible: rest.clone() }],
            may_shuffle: false,
        };
        if let Answer::Placements { placements, .. } = g.ask(you, ask)?
            && placements.first().is_some_and(|p| p.len() == rest.len())
        {
            order = placements[0].clone();
        }
    }
    g.arrange_library(you, &[], &order);
    Ok(())
}

/// Ripple `N` itself; `text` is the ability's own text.
pub const fn trigger<const N: usize>(text: &'static str) -> AbilityDef {
    triggered(crate::triggers::CAST_THIS_SPELL, ripple::<N>, text)
}

/// What ripple asks: whether to reveal, the cards to cast chosen after.
const REVEAL_FOR_RIPPLE: Question = Question::new(
    "RevealForRipple",
    "reveal the top cards of your library for ripple?",
    "Cast any with this spell's name without paying their mana costs  \u{b7}  the rest go on the bottom",
    "Reveal",
    "Don't reveal",
);
