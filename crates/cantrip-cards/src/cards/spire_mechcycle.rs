use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{is_mount_or_vehicle, mounts_and_vehicles_you_control};
use cantrip_oracle as oracle;

/// "Tap another untapped Mount or Vehicle you control" (CR 118.3, 205.3): a Mount or a Vehicle, untapped, that its controller
/// controls and that is not this Vehicle. It is not the {T} symbol of that permanent (CR 302.6): a creature that has not been under
/// its control since the start of the turn can be tapped for it.
fn another_untapped_mount_or_vehicle_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id != asker.source && q.controller(id) == asker.controller && !q.obj(id).tapped && is_mount_or_vehicle(q, asker.source, id)
}

/// "This Vehicle becomes an artifact creature. Put a +1/+1 counter on it for each Mount and/or Vehicle you control other than this
/// Vehicle." With no duration said, it stays a creature for as long as it is on the battlefield (CR 611.2a). The count is of
/// the permanents there are as it resolves, each once; a Vehicle that has left the battlefield by then is a new object that
/// gets nothing (CR 400.7).
fn becomes_a_creature_with_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others = mounts_and_vehicles_you_control(g, item.object, item.controller).into_iter().filter(|&id| id != item.object).count() as i32;
    g.add_effect(item.object, item.object, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), Duration::Indefinite);
    g.end_sentence();
    g.add_counters(item.object, CounterKind::PlusOne, others);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            tap_untapped(1, another_untapped_mount_or_vehicle_you_control),
            becomes_a_creature_with_counters,
            "Exhaust \u{2014} Tap another untapped Mount or Vehicle you control: This Vehicle becomes an artifact creature. Put a +1/+1 counter on it for each Mount and/or Vehicle you control other than this Vehicle. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::SPIRE_MECHCYCLE
};
