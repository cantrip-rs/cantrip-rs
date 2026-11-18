use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "Investigate." (CR 701.16a)
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// Haste is a core keyword. "Whenever you attack with three or more creatures, investigate" (CR 508.1, 603.2, 701.16a): once for the
/// whole attack, however many creatures attack, and the Youths need not be one of them.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKERS_DECLARED, triggers::you_attack_with_at_least::<3>),
        investigate,
        "Whenever you attack with three or more creatures, investigate.",
    )],
    ..oracle::MEDDLING_YOUTHS
};
