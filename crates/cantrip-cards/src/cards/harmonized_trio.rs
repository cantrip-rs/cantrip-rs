use cantrip_api::*;
use cantrip_mechanics::{another_untapped_creature_you_control, prepare};
use cantrip_oracle as oracle;

/// "{T}, Tap two untapped creatures you control": the other creatures, since this one is tapped for the {T} (CR 118.3, 302.6: a
/// summoning-sick creature may be tapped for another's cost).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(tap_untapped(2, another_untapped_creature_you_control)),
        prepare::become_prepared,
        "{T}, Tap two untapped creatures you control: This creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
    )],
    prepare: Some(&crate::brainstorm::CARD),
    ..oracle::HARMONIZED_TRIO
};
