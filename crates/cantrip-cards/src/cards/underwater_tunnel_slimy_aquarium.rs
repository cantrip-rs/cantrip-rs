//! Underwater Tunnel // Slimy Aquarium, a Room (CR 709.5): each door is only what its text does; the engine gives the Room
//! the abilities of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::{effects, manifest_dread, triggers};
use cantrip_oracle as oracle;

/// Underwater Tunnel: "When you unlock this door, surveil 2."
pub static UNDERWATER_TUNNEL: CardDef = CardDef {
    abilities: &[triggered(triggers::YOU_UNLOCK_THIS_LEFT_DOOR, |g, item| effects::surveil(g, item, 2), "When you unlock this door, surveil 2.")],
    ..oracle::UNDERWATER_TUNNEL
};

/// Slimy Aquarium: "When you unlock this door, manifest dread, then put a +1/+1 counter on that creature." The counter goes
/// on the face-down permanent the manifest made (CR 701.62a); with an empty library there is none, and nothing is put.
fn manifest_dread_then_plus_one_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = manifest_dread::manifest_dread_object(g, item.controller, item.object)? {
        g.add_counters(creature, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static SLIMY_AQUARIUM: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_UNLOCK_THIS_RIGHT_DOOR,
        manifest_dread_then_plus_one_counter,
        "When you unlock this door, manifest dread, then put a +1/+1 counter on that creature.",
    )],
    ..oracle::SLIMY_AQUARIUM
};

pub static CARD: CardDef = CardDef { split: Some((&UNDERWATER_TUNNEL, &SLIMY_AQUARIUM)), ..oracle::UNDERWATER_TUNNEL_SLIMY_AQUARIUM };

pub static EXTRA: &[&CardDef] = &[&UNDERWATER_TUNNEL, &SLIMY_AQUARIUM];
