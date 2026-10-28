use cantrip_api::*;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Seismic Takedown — Whenever you cast a noncreature spell, tap target creature or land." (CR 603.2, 701.26a): the target is
/// chosen as the ability goes on the stack, any creature or land on the battlefield whoever controls it, and tapped if it is
/// still a legal target as the ability resolves (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        NONCREATURE_SPELL_YOU_CAST,
        effects::tap_targets,
        "Seismic Takedown \u{2014} Whenever you cast a noncreature spell, tap target creature or land.",
    )
    .targets(&[targets::CREATURE_OR_LAND])],
    ..oracle::QUAKE_AGENT_OF_S_H_I_E_L_D
};
