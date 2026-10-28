//! Transfigure [cost] (CR 702.71a): "[Cost], Sacrifice this permanent: Search your library for a creature card with the same
//! mana value as this permanent and put it onto the battlefield. Then shuffle your library. Activate only as a sorcery." The
//! sacrificed permanent is read as it last existed on the battlefield (CR 608.2h). Fleshwrither prints it.
//!
//! A card with transfigure lists `transfigure(cost, text)` among its `abilities`; `text` is the ability's full text, which a
//! scenario's `Ability:` matches.

use crate::{library, select};
use cantrip_api::*;

/// The search: a creature card, which may be left unfound (CR 701.23b), whose mana value is the sacrificed permanent's
/// (CR 202.3), onto the battlefield untapped.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::creature_cards(select::up_to(1)).with(select::mana_value_exactly(g.mana_value(item.object)));
    library::find_to_battlefield(g, item.controller, item.object, rule, false)?;
    Ok(())
}

/// Transfigure `mana` (CR 702.71a), from the battlefield, at sorcery speed.
pub const fn transfigure(mana: ManaCost, text: &'static str) -> AbilityDef {
    ability(Cost::mana(mana).and(SACRIFICE_THIS), search, text).sorcery_speed()
}
