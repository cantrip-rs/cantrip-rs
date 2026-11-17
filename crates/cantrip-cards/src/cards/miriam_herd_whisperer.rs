use cantrip_api::*;
use cantrip_mechanics::{effects, is_mount_or_vehicle, statics};
use cantrip_oracle as oracle;

/// "During your turn, Mounts and Vehicles you control have hexproof" (CR 611.3a, 613.1f, 702.11b): the Mounts and Vehicles of
/// the source's controller, while it is that player's turn.
fn your_mounts_and_vehicles_on_your_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    q.state().turn.active == q.controller(source) && statics::mounts_and_vehicles_you_control(q, source, affected, chars)
}

/// "Whenever a Mount or Vehicle you control attacks" (CR 508.3a, 603.2c): each such creature declared as an attacker, in its own
/// event, is the trigger's subject; both subtypes are read through the words of Miriam's text (CR 612).
fn mount_or_vehicle_you_control_attacks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.controller(*attacker) == me.controller && is_mount_or_vehicle(q, me.source, *attacker) => {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        your_mounts_and_vehicles_on_your_turn,
        statics::have_hexproof,
        "During your turn, Mounts and Vehicles you control have hexproof.",
    )],
    abilities: &[triggered(
        when_asked(EventMask::ATTACKS, mount_or_vehicle_you_control_attacks),
        effects::plus_one_counter_on_that_permanent,
        "Whenever a Mount or Vehicle you control attacks, put a +1/+1 counter on it.",
    )],
    ..oracle::MIRIAM_HERD_WHISPERER
};
