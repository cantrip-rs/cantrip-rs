use cantrip_api::*;
use cantrip_mechanics::{effects, sneak, targets};
use cantrip_oracle as oracle;

/// "Choose one or both" (CR 700.2): each mode has its own target (CR 700.2c), which may be the same creature, and the modes are carried out in the
/// order printed (CR 700.2a). Sneak {W}{B} is the way to cast it listed first.
static MODAL: ModalDef = modes(&[
    mode(effects::target_creature_gets::<3, 3>, "Target creature gets +3/+3 until end of turn.").targets(&[targets::CREATURE]),
    mode(effects::target_creature_gets::<-3, -3>, "Target creature gets -3/-3 until end of turn.").targets(&[targets::CREATURE]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ways: &[sneak::way(mana("{W}{B}"), "Sneak {W}{B}")],
    ..oracle::KARAIS_TECHNIQUE
};
