use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, effects, tokens};
use cantrip_oracle as oracle;

/// "Each player who controls the most creatures investigates. Then destroy all creatures." (CR 701.16a, 701.8a, 608.2c): the
/// creatures are counted as the spell resolves; every player tied for the most (so with
/// nobody controlling a creature each player controls the most) investigates. They investigate at the same time (CR 101.4), so
/// every Clue is created in one event, as investigating creates it (a predefined token, CR 111.10f, `tokens::investigate`): what
/// watches "one or more tokens enter" sees them once. Then, as a second instruction, every creature is destroyed at once. A Clue
/// is not a creature, so none of the Clues is destroyed, and regeneration and indestructible are the verb's business.
fn no_witnesses(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let counts: Vec<(PlayerId, usize)> = g.state().apnap().map(|p| (p, creatures_you_control(g, p))).collect();
    let most = counts.iter().map(|&(_, n)| n).max();
    let clue = g.card_id(&tokens::CLUE);
    let clues: Vec<(CardId, PlayerId)> = counts.into_iter().filter(|&(_, n)| Some(n) == most).map(|(player, _)| (clue, player)).collect();
    g.create_tokens_worded(&clues, false, None, Words::NONE)?;
    effects::destroy_all_creatures(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], no_witnesses), ..oracle::NO_WITNESSES };
pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
