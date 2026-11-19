use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Tap target creature an opponent controls and put three stun counters on it. Scry 2." The creature is the spell's only target, so
/// when it is no longer legal the spell does not resolve and there is no scry (CR 608.2b, the ruling). The counters go on a creature
/// that was already tapped too (CR 701.26a, 122.1d).
fn tap_stun_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_and_stun::<3>(g, item)?;
    effects::scry_two(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_AN_OPPONENT_CONTROLS], tap_stun_and_scry), ..oracle::FREEZE_IN_PLACE };
