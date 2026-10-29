use cantrip_api::*;
use cantrip_mechanics::hand::discard_chosen_nonland_permanent_from_revealed_hand;
use cantrip_mechanics::{empower, targets};
use cantrip_oracle as oracle;

/// "Target opponent reveals their hand. You choose a nonland permanent card from it. That player discards that card."
/// (CR 701.9a, 110.4a): any nonland permanent card — an artifact, battle, creature, enchantment or planeswalker card, never
/// a land — may be taken, and nothing else. The hand is revealed and, when it holds no such card, no choice is made
/// (CR 608.2d). An opponent who is no longer a legal target stops the whole spell, empower included (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    discard_chosen_nonland_permanent_from_revealed_hand(g, item, opponent)?;
    // "Empower Jace 1" is the last instruction, so it happens after the discard (CR 608.2c).
    empower::jace(g, item.controller, item.object, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::OPPONENT], effect), ..oracle::SOLVE_FOR_DISAPPOINTMENT };

pub static EXTRA: &[&CardDef] = &[&empower::JACE];
