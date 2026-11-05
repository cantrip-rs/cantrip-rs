//! Devour (CR 702.82a), a static ability: "As this object enters, you may sacrifice any number of [permanents of a kind]. This
//! permanent enters with N +1/+1 counters on it for each permanent sacrificed this way." It is a replacement effect of the
//! entry (CR 614.12, 614.1c), so the sacrifice is made as it enters, before it is on the battlefield, and it can't devour
//! itself. Devour land (CR 702.82c; Famished Worldsire's ruling, 2025-07-25) is the same with lands in place of creatures; "you may choose
//! not to sacrifice any".
//!
//! A card lists `lands::<N>` as its `enters`. The count of what was sacrificed is what the player's permanents lost: a land
//! that a replacement keeps from being sacrificed is not counted (CR 702.82a: "sacrificed this way").

use crate::{choose, select};
use cantrip_api::*;

/// Devour land N: the player chooses any number of the lands they control, none included, and sacrifices them together; the
/// permanent enters with N +1/+1 counters for each land actually sacrificed.
pub fn lands<const N: i16>(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let you = entering.controller;
    let lands: Vec<ObjectId> = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    let chosen = choose::select(g, you, entering.object, Purpose::Sacrifice, lands, select::land_cards(select::ANY_NUMBER))?;
    if chosen.is_empty() {
        return Ok(());
    }
    let sacrificed = i64::try_from(g.sacrifice(you, &chosen).len()).expect("sacrificed objects fit the native count domain");
    entering.counters[CounterKind::PlusOne as usize] += i64::from(N) * sacrificed;
    Ok(())
}
