use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// Vigilance is a core keyword. "Whenever you attack with three or more creatures, put a +1/+1 counter on target creature you
/// control" (CR 508.1, 603.2, 122.1): once for the whole attack, however many creatures attack, and the target may be the Guide.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKERS_DECLARED, triggers::you_attack_with_at_least::<3>),
        effects::plus_one_counter_on_target,
        "Whenever you attack with three or more creatures, put a +1/+1 counter on target creature you control.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::ARMASAUR_GUIDE
};
