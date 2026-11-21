//! Firebending N (CR 702.189a): "Whenever this creature attacks, add N {R}. This mana lasts until end of combat."
//! Avatar Roku and the Dragon token his Saga's back face makes print it with 4.
//!
//! A card with firebending lists `trigger::<N>(text)` among its `abilities`.

use cantrip_api::*;

fn attacks(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == source => Fired::PLAIN,
        _ => None,
    }
}

fn firebend<const N: u8>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana_until_end_of_combat(item.controller, Mana::R, u32::from(N));
    Ok(())
}

/// `text` is the card's own reminder text.
pub const fn trigger<const N: u8>(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::ATTACKS, attacks), firebend::<N>, text).produces_mana()
}

/// "Firebending X" (CR 702.189a): `firebend_x` is the card's own effect, which works X out as the ability resolves (CR 608.2h)
/// and adds it with `add_x`. `text` is the card's own reminder text. Sun Warriors prints it with the number of creatures you
/// control.
pub const fn trigger_x(firebend_x: Effect, text: &'static str) -> AbilityDef {
    triggered(when(EventMask::ATTACKS, attacks), firebend_x, text).produces_mana()
}

/// Adds X {R} until end of combat for `you`: nothing for an X of 0 or less.
pub fn add_x(g: &mut dyn Game, you: PlayerId, x: i64) {
    if x > 0 {
        g.add_mana_until_end_of_combat(you, Mana::R, x as u32);
    }
}

/// "Firebending X, where X is this creature's power": X is the creature's power as the ability resolves (CR 608.2h), as it last
/// existed on the battlefield if it has left.
fn firebend_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.chars(item.object).power.unwrap_or(0);
    add_x(g, item.controller, i64::from(power));
    Ok(())
}

/// Firebending X, where X is the creature's power (Firebending Student). `text` is the card's own reminder text.
pub const fn trigger_its_power(text: &'static str) -> AbilityDef {
    trigger_x(firebend_its_power, text)
}
