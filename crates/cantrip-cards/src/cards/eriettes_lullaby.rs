use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target tapped creature. You gain 2 life." (CR 701.8, 119.3, 115.1): the creature must be tapped when chosen and again
/// as the spell resolves; the life is gained whether or not it was destroyed (indestructible, regenerated), but a target that is
/// gone or untapped by then makes the spell do nothing at all (CR 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::TAPPED_CREATURE], effects::destroy_first_target_and_gain_life::<2>),
    ..oracle::ERIETTES_LULLABY
};
