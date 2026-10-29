use cantrip_api::*;
use cantrip_mechanics::{effects::tap_targets, targets, TAP_ANOTHER_UNTAPPED_CREATURE};
use cantrip_oracle as oracle;

/// "{1}, {T}, Tap another untapped creature you control: Tap target creature an opponent controls." (CR 701.26a, 118.3): tapping
/// the other creature is a cost, so a summoning-sick creature may be tapped for it (it is not the {T} symbol, CR 107.5), and it
/// can't be the Trapper itself, which is tapped by {T}.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(TAP).and(TAP_ANOTHER_UNTAPPED_CREATURE),
        tap_targets,
        "{1}, {T}, Tap another untapped creature you control: Tap target creature an opponent controls.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::WANDERBRINE_TRAPPER
};
