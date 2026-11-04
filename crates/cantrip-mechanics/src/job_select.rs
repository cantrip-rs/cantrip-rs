//! Job select (CR 702.182a): "When this Equipment enters, create a 1/1 colorless Hero creature token, then attach this
//! Equipment to it." Astrologian's Planisphere and Dark Knight's Greatsword print it.
//!
//! A card with job select lists `JOB_SELECT` among its `abilities` and `HERO` among its `EXTRA` definitions.

use cantrip_api::*;

use crate::{tokens, triggers};

/// The 1/1 colorless Hero creature token job select makes (`tokens::HERO_1_1_COLORLESS`).
pub use crate::tokens::HERO_1_1_COLORLESS as HERO;

/// "Create a 1/1 colorless Hero creature token, then attach this to it." An Equipment that has left the battlefield attaches
/// to nothing, and a token a replacement stopped is not there to be equipped (CR 701.3b). The token enters first, as a 1/1
/// creature, and is then equipped (the rulings). The Hero is the keyword's (CR 702.182a), which no text change of the
/// Equipment's reaches (CR 207.2a, 612.2): Artificial Evolution changing Hero to Elf still gets a Hero.
fn job_select(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create_by_rule(g, item.controller, &HERO, 1, false)?;
    if let Some(&token) = made.first()
        && is_live(g, token)
        && g.is_on_battlefield(token)
        && is_live(g, item.object)
        && g.is_on_battlefield(item.object)
    {
        g.attach(item.object, token);
    }
    Ok(())
}

/// Job select (CR 702.182a), for a card's `abilities`.
pub const JOB_SELECT: AbilityDef = triggered(
    triggers::THIS_ENTERS,
    job_select,
    "Job select (When this Equipment enters, create a 1/1 colorless Hero creature token, then attach this to it.)",
);
