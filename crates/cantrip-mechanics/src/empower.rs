//! Empower Jace N (CR 701.71a): "If you don't control a Jace planeswalker token, create a blue Jace planeswalker token
//! with 0 loyalty, '[-1]: Surveil 1,' and '[-3]: Draw a card.' Choose a Jace planeswalker token you control. Put N
//! loyalty counters on it." A nontoken Jace planeswalker is never chosen. Theorist's Sanctum, Fatehold Charm,
//! Countersculpt, Jace's Machinations, Avatar of Burgeoning Echoes and Sanctum Lurker print it.

use crate::{choose, effects, tokens};
use cantrip_api::*;

fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// The blue Jace planeswalker token empower Jace makes, entering with no loyalty counters. Its card lists it in its `EXTRA`.
pub static JACE: CardDef = CardDef {
    name: "Jace Token",
    token_name: Some(&[Subtype::Jace]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Planeswalker]),
    subtypes: SubtypeSet::of(&[Subtype::Jace]),
    loyalty: Some(0),
    abilities: &[
        ability(loyalty(-1), surveil_one, "\u{2212}1: Surveil 1."),
        ability(loyalty(-3), draw_a_card, "\u{2212}3: Draw a card."),
    ],
    text: "\u{2212}1: Surveil 1.\n\u{2212}3: Draw a card.",
    ..CardDef::BLANK
};

fn jace_tokens(g: &dyn Game, controller: PlayerId) -> Vec<ObjectId> {
    g.permanents_of(controller)
        .into_iter()
        .filter(|&id| g.obj(id).token && g.chars(id).is(CardType::Planeswalker) && g.chars(id).has_subtype(Subtype::Jace))
        .collect()
}

/// "Empower Jace N" for `controller`, `source` being the object whose ability does it. With several Jace planeswalker
/// tokens (a token copy of a Jace keeps its printed loyalty) the player chooses which gets the counters, a choice that
/// is not targeting (`Purpose::PutCounters`).
pub fn jace(g: &mut dyn Game, controller: PlayerId, source: ObjectId, n: i16) -> Result<(), Suspend> {
    let mut owned = jace_tokens(g, controller);
    if owned.is_empty() {
        tokens::create(g, controller, &JACE, 1)?;
        owned = jace_tokens(g, controller);
    }
    let jace = match owned.as_slice() {
        [] => None,
        [only] => Some(*only),
        _ => choose::cards_from_kind(g, controller, source, Purpose::PutCounters(CounterKind::Loyalty, n.clamp(0, 255) as u8), "Jace planeswalker token", owned.clone(), owned, 1, false)?.first().copied(),
    };
    if let Some(jace) = jace {
        g.add_counters(jace, CounterKind::Loyalty, n);
    }
    Ok(())
}
