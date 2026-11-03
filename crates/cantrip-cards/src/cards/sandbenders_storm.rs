use cantrip_api::*;
use cantrip_mechanics::{earthbend, effects, targets};
use cantrip_oracle as oracle;

/// Choose one mode as the spell is cast (CR 700.2a); each mode has its own target (CR 700.2c).
static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy target creature with power 4 or greater.").targets(&[targets::CREATURE_WITH_POWER_4_OR_GREATER]),
    mode(earthbend::effect::<3>, "Earthbend 3.").targets(&[earthbend::TARGET]),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    delayed: &[earthbend::RETURN],
    ..oracle::SANDBENDERS_STORM
};
