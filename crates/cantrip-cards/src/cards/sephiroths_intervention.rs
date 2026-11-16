use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature. You gain 2 life." (CR 701.8, 119.3): the life is gained whether or not the creature was destroyed
/// (an indestructible one stays), as long as the spell resolved; if its only target is gone it does not resolve (CR 608.2b)
/// and no life is gained.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::destroy_first_target_and_gain_life::<2>),
    ..oracle::SEPHIROTHS_INTERVENTION
};
