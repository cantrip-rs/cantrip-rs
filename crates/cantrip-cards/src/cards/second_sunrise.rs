use cantrip_api::*;
use cantrip_mechanics::history::put_into_graveyard_from_battlefield_this_turn;
use cantrip_oracle as oracle;

/// Each player returns to the battlefield all artifact, creature, enchantment, and land cards in their graveyard that
/// were put there from the battlefield this turn: every player's, together, each under its owner's control.
fn effect(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let mut back: Vec<(ObjectId, PlayerId)> = Vec::new();
    let players: Vec<PlayerId> = g.state().apnap().collect();
    for player in players {
        for id in put_into_graveyard_from_battlefield_this_turn(g, player) {
            let c = g.chars(id);
            if [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Land].iter().any(|&t| c.is(t)) {
                back.push((id, player));
            }
        }
    }
    if !back.is_empty() {
        g.put_all_onto_battlefield_by(&back.into_iter().map(|(card, player)| (card, player, false, player)).collect::<Vec<_>>(), &[])?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::SECOND_SUNRISE };
