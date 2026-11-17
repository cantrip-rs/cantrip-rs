use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature gains deathtouch and lifelink until end of turn. Investigate." (CR 702.2, 702.15, 611.2a, 701.16a): both
/// keywords from one layer 6 effect on the target; a target no longer legal as the spell resolves means the spell does not
/// resolve and nothing, the Clue included, is made (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Deathtouch, Keyword::Lifelink]).0 }>(g, item)?;
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::TOXIN_ANALYSIS
};
