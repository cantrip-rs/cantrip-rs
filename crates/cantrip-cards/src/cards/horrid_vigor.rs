use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gains deathtouch and indestructible until end of turn." (CR 702.2, 702.12, 611.2a): both keywords from one
/// layer 6 effect on the target, which does nothing if it is no longer a legal target as the spell resolves (CR 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE],
        effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Deathtouch, Keyword::Indestructible]).0 }>,
    ),
    ..oracle::HORRID_VIGOR
};
