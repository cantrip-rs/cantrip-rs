use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Each player shuffles the cards from their hand into their library, then draws that many cards." (CR 608.2, 121.1).
/// Every hand goes into its owner's library at once; a player with no card in hand still shuffles (CR 701.24d). Each player then
/// draws as many cards as they shuffled in, in turn order.
fn player_shuffles_cards_their_hand(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    let hands: Vec<ObjectId> = players
        .iter()
        .flat_map(|&p| g.state().zone(Zone::hand(p)).iter().copied())
        .filter(|&id| !g.state().obj(id).token && !g.state().obj(id).card_copy)
        .collect();
    g.put_into_library(&hands, LibraryPlace::ShuffledIn);
    for &player in &players {
        if !hands.iter().any(|&id| g.state().obj(id).owner == player) {
            g.shuffle(player);
        }
    }
    g.end_sentence();
    for &player in &players {
        let shuffled = hands
            .iter()
            .filter(|&&id| g.state().obj(id).became.is_some() && g.state().obj(id).owner == player)
            .count() as u32;
        if g.state().in_game(player) {
            g.draw(player, shuffled);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], player_shuffles_cards_their_hand),
    ..oracle::WINDS_OF_CHANGE
};
