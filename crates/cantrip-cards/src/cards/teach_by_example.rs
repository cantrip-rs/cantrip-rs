use cantrip_api::*;
use cantrip_mechanics::{copy, delayed};
use cantrip_oracle as oracle;

/// "When you next cast an instant or sorcery spell this turn, copy that spell": resolving, the instant sets up a delayed triggered
/// ability that triggers once on the next such spell its controller casts this turn and ends with the turn if none is cast
/// (CR 603.7b). Teach by Example is itself already cast, so it is not that spell.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], delayed::watch_for_the_next),
    delayed: &[copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST],
    ..oracle::TEACH_BY_EXAMPLE
};
