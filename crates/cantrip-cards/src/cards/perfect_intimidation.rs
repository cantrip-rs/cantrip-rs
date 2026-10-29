use cantrip_api::*;
use cantrip_mechanics::{effects, exile_chosen_from_hand, targets};
use cantrip_oracle as oracle;

/// "Target opponent exiles two cards from their hand": the opponent chooses them (CR 608.2d), all of the hand if it has
/// fewer; nothing if they are no longer a legal target as it resolves (CR 608.2b).
fn exile_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(opponent) = targets::player(g, item, 0) {
        exile_chosen_from_hand(g, opponent, item.object, 2)?;
    }
    Ok(())
}

/// "Remove all counters from target creature": every kind it has (CR 122.1, 122.8).
fn remove_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        effects::remove_all_counters(g, creature);
    }
    Ok(())
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 608.2c).
static MODAL: ModalDef = modes(&[
    mode(exile_two, "Target opponent exiles two cards from their hand.").targets(&[targets::OPPONENT]),
    mode(remove_counters, "Remove all counters from target creature.").targets(&[targets::CREATURE]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::PERFECT_INTIMIDATION };
