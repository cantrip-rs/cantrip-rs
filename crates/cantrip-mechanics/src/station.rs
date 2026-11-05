//! Station (CR 702.184a): "Tap another untapped creature you control: Put a number of charge counters on this permanent
//! equal to the tapped creature's power. Activate only as a sorcery." A static ability may make it use the creature's
//! toughness instead (CR 702.184c, `CardDef::stations_by_toughness`). The station symbols of a station card (CR 721.2) are
//! its own statics, read from its charge counters with `charged::<N>`: "{N+}[abilities]" means "as long as this permanent
//! has N or more charge counters on it, it has [abilities]" (CR 721.2a), and "{N+}[abilities][P/T]" makes it also "a
//! creature with base power and toughness [P/T] in addition to its other types" (CR 721.2b, `creature::<P, T>`).
//!
//! Every station card prints it: the Planets (Adagia, Windswept Bastion; Uthros, Titanic Godcore; Evendo, Waking Haven)
//! and the Spacecraft (Wurmwall Sweeper, Galvanizing Sawship, ...).

use cantrip_api::*;

/// "Put a number of charge counters on this permanent equal to the tapped creature's power": its power as the ability
/// resolves, or as it last existed on the battlefield if it has left (CR 608.2h; the station rulings) - or its toughness, if
/// a static ability on the battlefield as it resolves makes it station using its toughness (CR 702.184c, Tapestry Warden),
/// judged of the creature as it last existed too. A value of 0 or less puts none and removes none (ruling 2025-07-25). A
/// station permanent that has left the battlefield gets none (`add_counters` skips an object that has changed zones: one
/// that came back is a new object, CR 400.7).
fn put_charge_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&tapped) = g.cost_objects(item).first() else { return Ok(()) };
    let chars = g.chars(tapped);
    let amount = if g.stations_using_toughness(tapped) { chars.toughness } else { chars.power }.unwrap_or(0);
    if amount > 0 {
        g.add_counters(item.object, CounterKind::Charge, i64::from(amount));
    }
    Ok(())
}

/// The station ability (CR 702.184a). Its cost taps a creature, not the {T} symbol (CR 107.5, 302.6), so a creature that
/// came under its controller's control this turn may station. `text` is the ability's full text, as printed.
pub const fn ability(text: &'static str) -> AbilityDef {
    cantrip_api::ability(crate::TAP_ANOTHER_UNTAPPED_CREATURE, put_charge_counters, text).sorcery_speed()
}

/// The number of charge counters on `source`, as the layers see it.
pub fn charge(view: &dyn LayerQuery, source: ObjectId) -> i16 {
    view.state().obj(source).counters(CounterKind::Charge)
}

/// A station symbol "{N+}" (CR 721.2a): the static ability of its striation applies to its own permanent while that
/// permanent has N or more charge counters on it. Use it as a static ability's `affects`.
pub fn charged<const N: i16>(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    affected == source && charge(view, source) >= N
}

/// "[P/T]" in a striation (CR 721.2b), layer 4: it is a creature in addition to its other types.
pub use crate::statics::creature_too as is_a_creature;

/// The abilities of a striation (CR 721.2a), layer 6: the ability at `INDEX` of the station card's `delayed` list. A
/// permanent that copies the station card has it too, read from its own charge counters (ruling 2025-07-25).
pub use crate::statics::has_delayed_ability as has;
