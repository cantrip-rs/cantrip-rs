use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): Blow Off Steam deals 1 damage to any target.
pub static BLOW_OFF_STEAM: CardDef = CardDef { spell: spell(&[targets::ANY], effects::damage_first_target::<1>), ..oracle::BLOW_OFF_STEAM };

/// Flying is a core keyword. "Whenever you cast an instant or sorcery spell, this creature gets +1/+1 until end of turn." Each such
/// spell adds its own, and they stack.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        effects::this_gets::<1, 1>,
        "Whenever you cast an instant or sorcery spell, this creature gets +1/+1 until end of turn.",
    )],
    adventure: Some(&BLOW_OFF_STEAM),
    ..oracle::FROLICKING_FAMILIAR
};

pub static EXTRA: &[&CardDef] = &[&BLOW_OFF_STEAM];
