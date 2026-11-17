use cantrip_api::*;
use cantrip_mechanics::statics::{itself_while_you_committed_a_crime, plus_pt};
use cantrip_oracle as oracle;

/// Vigilance (CR 702.20). "This creature gets +2/+0 as long as you've committed a crime this turn" (CR 613.4c, 700.13): asked again
/// whenever its power is computed, so it ends with the turn.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself_while_you_committed_a_crime,
        plus_pt::<2, 0>,
        "This creature gets +2/+0 as long as you've committed a crime this turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )],
    ..oracle::SLICKSHOT_VAULT_BUSTER
};
