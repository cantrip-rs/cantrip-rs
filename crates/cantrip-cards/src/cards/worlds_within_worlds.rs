use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "Exile all creatures. Each player may put any number of creature cards from their hand onto the battlefield. Then put
/// all cards exiled this way into their owners' hands. Exile Worlds Within Worlds."
///
/// Every creature goes at once (CR 406.2, 608.2c). Each player then chooses in turn order from the active player (CR 101.4),
/// one not seeing another's choice, and all the chosen cards enter together, each under its chooser's control. "All cards
/// exiled this way" are the objects the first instruction exiled - a token that was exiled has ceased to exist and is not
/// returned (CR 111.7, 400.7) - and the creatures just put onto the battlefield are new objects, so they stay. The spell
/// exiles itself last, so it leaves the stack for nowhere more when it finishes (CR 608.2m).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let exiled = g.exile_all(&creatures);
    let mut entering = Vec::new();
    for player in g.state().apnap().collect::<Vec<_>>() {
        if !cantrip_mechanics::players::reached(g, player) {
            continue;
        }
        let hand = g.state().zone(Zone::hand(player)).to_vec();
        for card in choose::select(g, player, item.object, Purpose::PutOntoBattlefield, hand, select::creature_cards(select::ANY_NUMBER))? {
            entering.push((card, player));
        }
    }
    g.put_all_onto_battlefield(&entering)?;
    g.return_to_hand(&exiled);
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::WORLDS_WITHIN_WORLDS };
