use cantrip_api::*;
use cantrip_mechanics::effects::gain_life_then_draw;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_oracle as oracle;

/// "Whenever Wylie Duke becomes tapped" (CR 701.26a, 603.2): it is untapped, then tapped by an effect or a cost - saddling a Mount,
/// crewing, a tap effect - and not when it attacks, which vigilance leaves untapped (CR 702.20b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_BECOMES_TAPPED, gain_life_then_draw::<1, 1>, "Whenever Wylie Duke becomes tapped, you gain 1 life and draw a card.")],
    ..oracle::WYLIE_DUKE_ATIIN_HERO
};
