//! Scavenge [cost] (CR 702.97a): "[Cost], Exile this card from your graveyard: Put a number of +1/+1 counters equal to
//! the power of the card you exiled on target creature. Activate only as a sorcery." Deadbridge Goliath, Dreg Mangler,
//! Drudge Beetle, Korozda Monitor, Sewer Shambler, Slitherhead, Sluiceway Scorpion, Terrus Wurm, Thrashing Mossdog and
//! Zanikev Locust print it.
//!
//! Exiling the card is part of the cost, so the ability exists whatever happens to its target; the number of counters is
//! the power the card had in the graveyard (CR 608.2h: the exiled card's last-known information, with no layers of the
//! battlefield), and none if the creature is no longer a legal target (CR 608.2b). A card that is given scavenge by
//! another object (Varolz, the Scar-Striped) is read the same way, whatever cost it is given.
//!
//! A card with scavenge lists `ability(cost, text)` among its `abilities`.

use crate::targets;
use cantrip_api::*;

/// "Put a number of +1/+1 counters equal to the power of the card you exiled on target creature." The card is the
/// ability's source, exiled to pay (a new object now), so `chars` gives the power it last had in the graveyard (CR 400.7,
/// 608.2h); a power below zero puts none.
fn put_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let power = g.chars(item.object).power.unwrap_or(0).max(0);
    g.add_counters(creature, CounterKind::PlusOne, power);
    Ok(())
}

/// The scavenge ability, activated from the graveyard at sorcery speed, targeting any creature.
pub const fn ability(cost: ManaCost, text: &'static str) -> AbilityDef {
    cantrip_api::ability(Cost::mana(cost).and(EXILE_THIS), put_counters, text)
        .sorcery_speed()
        .from(ZoneKind::Graveyard)
        .targets(&[targets::CREATURE])
}
