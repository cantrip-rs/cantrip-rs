use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, targets};
use cantrip_oracle as oracle;

/// "Target opponent exiles an enchantment they control." The opponent chooses the enchantment as the spell resolves
/// (CR 608.2d), among those they control then, asked even when there is only one (as Strategic Betrayal's exile is); with
/// none nothing happens. The target is the player, so hexproof on a permanent doesn't matter; if the opponent is no longer
/// a legal target the mode does nothing (CR 608.2b).
fn opponent_exiles_an_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let enchantments: Vec<ObjectId> = g.permanents_of(opponent).into_iter().filter(|&id| g.chars(id).is(CardType::Enchantment)).collect();
    let chosen = choose::select(g, opponent, item.object, Purpose::Exile, enchantments, select::cards(select::exactly(1)))?;
    g.exile_all(&chosen);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::exile_first_target, "Exile target creature.").targets(&[targets::CREATURE]),
    mode(opponent_exiles_an_enchantment, "Target opponent exiles an enchantment they control.").targets(&[targets::OPPONENT]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::EARLY_WINTER };
