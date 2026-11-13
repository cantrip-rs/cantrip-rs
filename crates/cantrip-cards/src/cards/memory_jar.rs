use cantrip_api::*;
use cantrip_mechanics::delayed;
use cantrip_oracle as oracle;

/// Each player exiles all cards from their hand, face down, and draws seven cards: every hand leaves at once (CR 101.4),
/// the cards remembered as exiled with the jar, which has been sacrificed by now but is still the object the ability
/// came from. At the beginning of the next end step, of anyone's turn, the cards come back.
///
/// Face down (CR 406.3): each hand's owner may look at what was theirs, nobody else may.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<PlayerId> = g.state().apnap().collect();
    let hands: Vec<ObjectId> = players.iter().flat_map(|&p| g.state().zone(Zone::hand(p)).to_vec()).collect();
    g.exile_face_down(&hands, Some(g.linked(item)));
    for &p in &players {
        g.draw(p, 7);
    }
    delayed::at_next_end_step(g, item, 0, Target::None);
    Ok(())
}

/// Each player discards their hand - every hand at once, one discard (CR 101.4, 701.9) - and returns to their hand each card
/// they exiled this way: the cards still in exile that were exiled with the jar (a card that has left exile since is another
/// object, CR 400.7).
fn give_back(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hands: Vec<ObjectId> = g.state().apnap().flat_map(|p| g.state().zone(Zone::hand(p)).to_vec()).collect();
    g.discard_together(&hands);
    let exiled = g.exiled_with(g.linked(item));
    g.return_to_hand(&exiled);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP.and(SACRIFICE_THIS), effect, "{T}, Sacrifice this artifact: Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way.")],
    delayed: &[delayed_trigger(give_back, "Each player discards their hand and returns to their hand each card they exiled this way.")],
    ..oracle::MEMORY_JAR
};
