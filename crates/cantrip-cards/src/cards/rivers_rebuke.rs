use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return all nonland permanents target player controls to their owner's hand." (CR 400.3): the permanents that player controls
/// as it resolves, together (CR 608.2f), a token among them ceasing to exist (CR 111.7); nothing if the player is no longer a legal
/// target (CR 608.2b). They go to their owners' hands, so a permanent the player controls but does not own does not go to theirs.
fn return_nonland_permanents(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let nonland: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| !g.chars(id).is(CardType::Land)).collect();
    g.return_to_hand(&nonland);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::PLAYER], return_nonland_permanents), ..oracle::RIVERS_REBUKE };
