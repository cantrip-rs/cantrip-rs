use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_oracle as oracle;

const SHUFFLE_IN: Question = Question::new("ShuffleIn", "shuffle your hand and graveyard into your library?", "Chosen as the spell resolves", "Shuffle", "Don't");

/// "Return all creatures to their owners' hands. Each player may shuffle their hand and graveyard into their library, then
/// each player who does draws seven cards. Exile Turtles in Time." The creatures return at once as the spell resolves
/// (CR 400.7), so they are in their owners' hands when the players choose and can be shuffled in. Each player in turn order
/// from the active player chooses, knowing the choices before theirs (CR 101.4); the hands and graveyards of those who said
/// yes move together (CR 701.24), their libraries are shuffled, and each of them draws seven in turn order. The spell
/// exiles itself last (CR 608.2m), so it is on the stack, not in a graveyard, while the players shuffle.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.state().battlefield().into_iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    g.return_to_hand(&creatures);
    g.end_sentence();
    let players: Vec<PlayerId> = g.state().apnap().collect();
    let mut shuffling: Vec<PlayerId> = Vec::new();
    for &p in &players {
        if ask_may(g, p, item.object, Purpose::Ask(&SHUFFLE_IN))? {
            shuffling.push(p);
        }
    }
    let mut cards: Vec<ObjectId> = Vec::new();
    for &p in &shuffling {
        cards.extend(g.state().zone(Zone::hand(p)).iter().chain(g.state().zone(Zone::graveyard(p)).iter()).copied());
    }
    g.put_into_library(&cards, LibraryPlace::Top);
    for &p in &shuffling {
        g.shuffle(p);
    }
    for &p in &shuffling {
        g.draw(p, 7);
    }
    g.end_sentence();
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::TURTLES_IN_TIME };
