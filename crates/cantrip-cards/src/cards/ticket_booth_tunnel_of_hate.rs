//! Ticket Booth // Tunnel of Hate, a Room (CR 709.5): each door is only what its text does; the engine gives the Room the abilities
//! of its unlocked doors (`layers::doors`).

use cantrip_api::*;
use cantrip_mechanics::triggers::{YOU_ATTACK, YOU_UNLOCK_THIS_LEFT_DOOR};
use cantrip_mechanics::{effects, manifest_dread, targets};
use cantrip_oracle as oracle;

/// Ticket Booth: "When you unlock this door, manifest dread."
pub static TICKET_BOOTH: CardDef = CardDef {
    abilities: &[triggered(YOU_UNLOCK_THIS_LEFT_DOOR, manifest_dread::controller_manifests_dread, "When you unlock this door, manifest dread.")],
    ..oracle::TICKET_BOOTH
};

/// Tunnel of Hate: "Whenever you attack, target attacking creature gains double strike until end of turn." The target is chosen
/// as the ability goes on the stack (CR 603.3d), any attacking creature, whoever controls it.
pub static TUNNEL_OF_HATE: CardDef = CardDef {
    abilities: &[triggered(YOU_ATTACK, effects::target_gains_double_strike, "Whenever you attack, target attacking creature gains double strike until end of turn.")
        .targets(&[targets::ATTACKING_CREATURE])],
    ..oracle::TUNNEL_OF_HATE
};

pub static CARD: CardDef = CardDef { split: Some((&TICKET_BOOTH, &TUNNEL_OF_HATE)), ..oracle::TICKET_BOOTH_TUNNEL_OF_HATE };

pub static EXTRA: &[&CardDef] = &[&TICKET_BOOTH, &TUNNEL_OF_HATE];
