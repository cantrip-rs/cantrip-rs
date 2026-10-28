//! Riot (CR 702.136a), a static ability: "You may have this permanent enter with an additional +1/+1 counter on it. If you
//! don't, it gains haste." Ghor-Clan Wrecker, Rampaging Rendhorn, Wrecking Beast and Zhur-Taa Goblin print it. Each
//! instance works separately (CR 702.136b); a card prints it once.
//!
//! The choice is made as it enters (CR 614.12a) and kept with the text that made it (`choose::option_as_enters`), so a
//! copy of the permanent makes its own (CR 707.5). A card with riot has `enters` calling `enters(g, entering, &CARD)`,
//! `labels: LABELS` and `HASTE_STATIC` among its `statics`.

use crate::choose;
use crate::statics::{have_haste, itself};
use cantrip_api::*;

pub const LABELS: &[&str] = &["+1/+1 counter", "Haste"];
const COUNTER: u8 = 0;
const HASTE: u8 = 1;

/// "Enters with your choice of a +1/+1 counter or haste" (CR 614.1c, 122.6): the counter, if chosen, is on it as it
/// enters. `def` is the card whose text this is, the caller's `&CARD`. If a +1/+1 counter can't be put on it, it gains haste
/// (the ruling), so a choice of the counter that could not be carried out is the choice of haste.
pub fn enters(g: &mut dyn Game, entering: &mut Entering, def: &'static CardDef) -> Result<(), Suspend> {
    choose::option_as_enters(g, entering, def, 0..2)?;
    let Some(chosen) = entering.chosen_option.as_mut() else { return Ok(()) };
    if chosen.index == COUNTER {
        if g.counters_banned(entering.object, entering.controller, CounterKind::PlusOne) {
            chosen.index = HASTE;
        } else {
            entering.counters[CounterKind::PlusOne as usize] += 1;
        }
    }
    Ok(())
}

/// "If you don't, it gains haste": itself, if it was haste that was chosen. A choice made by another text (a copy effect
/// gave it this ability later) is no choice of this one (CR 607.5), which `Chosen::to` judges.
fn itself_if_haste_was_chosen(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    itself(q, source, affected, so_far) && q.state().chosen_option(q.linked_to(source)).is_some_and(|c| c.index == HASTE)
}

pub const HASTE_STATIC: StaticDef = static_ability(Layer::Ability, itself_if_haste_was_chosen, have_haste, "If you chose haste, this creature has haste.");
