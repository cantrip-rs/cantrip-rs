use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::has_delayed_ability;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// The 1/1 blue Fish creature token it creates.
pub static EXTRA: &[&CardDef] = &[&tokens::FISH];

/// "Put a bait counter on Fishing Pole." (CR 113.10, 201.5a, 122.1): the Fishing Pole that gave the creature this ability, kept
/// as it was activated (`item.instance.granter`; the ruling: another Pole on the same creature gets nothing). Once that Pole has
/// left the battlefield it is a new object (CR 400.7) and nothing is put anywhere.
fn put_a_bait_counter_on_the_pole(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let pole = item.instance.granter;
    if pole != ObjectId::NONE {
        g.add_counters(pole, CounterKind::Bait, 1);
    }
    Ok(())
}

/// "Whenever equipped creature becomes untapped" (CR 701.26b, 603.2): the creature this Equipment is attached to goes from
/// tapped to untapped. A creature that enters untapped, or is untapped already, does not trigger it.
fn equipped_creature_untaps(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Untapped { permanent } if q.attached_to(source) == Some(*permanent) => Fired::PLAIN,
        _ => None,
    }
}

/// "Remove a bait counter from this Equipment. If you do, create a 1/1 blue Fish creature token." (CR 608.2c, 122.1): the token
/// only if a counter came off, so none from a Pole without bait or one that has left the battlefield (CR 400.7).
fn remove_a_bait_counter_then_a_fish(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.remove_counters(item.object, CounterKind::Bait, 1) > 0 {
        tokens::create_a_fish(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::UNTAPPED, equipped_creature_untaps),
            remove_a_bait_counter_then_a_fish,
            "Whenever equipped creature becomes untapped, remove a bait counter from this Equipment. If you do, create a 1/1 blue Fish creature token.",
        ),
        equip(mana("{2}"), "Equip {2}"),
    ],
    // The ability the equipped creature has (CR 613.1f), its own, so its controller controls and pays for it (CR 113.8). "Tap
    // Fishing Pole" taps the Pole that gave this instance (CR 201.5a, 118.3), and no other Pole; the creature's {T} is its own.
    delayed: &[ability(
        cost("{1}").and(TAP).and(Cost { tap_granter: true, ..Cost::NONE }),
        put_a_bait_counter_on_the_pole,
        "{1}, {T}, Tap Fishing Pole: Put a bait counter on Fishing Pole.",
    )],
    statics: &[static_ability(
        Layer::Ability,
        attached_object,
        has_delayed_ability::<0>,
        "Equipped creature has \"{1}, {T}, Tap Fishing Pole: Put a bait counter on Fishing Pole.\"",
    )],
    ..oracle::FISHING_POLE
};
