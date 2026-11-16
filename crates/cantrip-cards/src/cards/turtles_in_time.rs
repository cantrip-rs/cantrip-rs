use cantrip_api::*;
use cantrip_mechanics::each_player_may_shuffle_hand_and_graveyard_in_and_draw_seven;
use cantrip_oracle as oracle;

/// "Return all creatures to their owners' hands. Each player may shuffle their hand and graveyard into their library, then
/// each player who does draws seven cards. Exile Turtles in Time." The creatures return at once as the spell resolves
/// (CR 400.7), so they are in their owners' hands when the players choose and can be shuffled in. The spell exiles itself
/// last (CR 608.2m), so it is on the stack, not in a graveyard, while the players shuffle.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.state().battlefield().into_iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    g.return_to_hand(&creatures);
    g.end_sentence();
    each_player_may_shuffle_hand_and_graveyard_in_and_draw_seven(g, item.object)?;
    g.end_sentence();
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::TURTLES_IN_TIME };
