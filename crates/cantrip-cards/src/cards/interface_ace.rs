use cantrip_api::*;
use cantrip_mechanics::{effects, is_mount_or_vehicle};
use cantrip_mechanics::triggers::this_becomes_tapped;
use cantrip_oracle as oracle;

/// "This creature saddles Mounts and crews Vehicles using its toughness rather than its power" (CR 702.171c, 702.122b): asked as
/// the creature is tapped for a crew or saddle cost, of the permanent being crewed or saddled, with the power it has then; its
/// toughness replaces that power, so what else changed it (a bonus from another ability) is kept. A Spacecraft it stations
/// still counts its power.
fn toughness_for_mounts_and_vehicles(q: &dyn Query, asker: &Asker, power: i32, permanent: ObjectId) -> i32 {
    if is_mount_or_vehicle(q, asker.source, permanent) {
        let this = q.chars(asker.source);
        power - i32::from(this.power.unwrap_or(0)) + i32::from(this.toughness.unwrap_or(0))
    } else {
        power
    }
}

/// "Whenever this creature becomes tapped during your turn": by any path that taps it, a crew or saddle cost included, while its
/// controller is the active player (CR 701.26a, 603.2).
fn this_becomes_tapped_during_your_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if q.state().turn.active == me.controller { this_becomes_tapped(q, me.source, event) } else { None }
}

pub static CARD: CardDef = CardDef {
    tap_power: &[toughness_for_mounts_and_vehicles],
    abilities: &[triggered(
        when_asked(EventMask::TAPPED, this_becomes_tapped_during_your_turn),
        effects::untap_self,
        "Whenever this creature becomes tapped during your turn, untap it. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::INTERFACE_ACE
};
