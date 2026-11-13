use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Tap target creature or Vehicle, then put three stun counters on it." (CR 701.26a, 122.1d): the counters go on whether or
/// not it was untapped (a Vehicle that is not a creature included), and with the target gone the spell does nothing (CR 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_VEHICLE], effects::tap_and_stun::<3>),
    abilities: &[triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::STALL_OUT
};
