use cantrip_api::*;
use cantrip_mechanics::{choose, goad};
use cantrip_mechanics::library;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

fn is_creature(q: &dyn Query, card: ObjectId) -> bool {
    q.chars(card).is(CardType::Creature)
}

/// "When Dack Fayden enters, reveal cards from the top of your library until you reveal X creature cards, where X is the
/// number of opponents you have. Put those creature cards onto the battlefield, then shuffle. They're goaded for the rest
/// of the game. For each of those permanents, choose a different opponent. Each opponent gains control of the permanent
/// for which they were chosen."
///
/// X is the opponents still in the game (CR 800.4a), and the reveal stops at the Xth creature card, or at the bottom of a
/// library holding fewer (CR 701.20a). The revealed cards are shown together and stay in the library, which the shuffle
/// that follows (CR 701.24) puts in a random order anyway; the creature cards found leave it for the battlefield as one
/// event (CR 608.2c). "Those permanents" are the ones that entered: a card a replacement sent elsewhere (Containment
/// Priest's exile) or a rule kept where it was (Weathered Runestone) is not one, so it is not goaded, no opponent is chosen
/// for it, and it uses none of them up. The goad is Dack's, so it lasts for the rest of the game, ending only with the
/// permanent it is on, should that leave the battlefield (CR 400.7) - the control change comes after it, so it is still the
/// ability's controller who goads, and one opponent each, never the same one twice, receives them. Each choice names the
/// permanent it is for.
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    // The revealed cards stay in the library, which the shuffle puts in a random order anyway.
    let (found, _) = library::reveal_until_n(g, you, opponents.len(), &is_creature);
    let entering: Vec<(ObjectId, PlayerId)> = found.iter().map(|&card| (card, you)).collect();
    let entered = g.put_all_onto_battlefield(&entering)?;
    let permanents: Vec<ObjectId> = entered.into_iter().filter(|&id| g.is_on_battlefield(id)).collect();
    g.shuffle(you);
    instruction(g, |g| {
        for &permanent in &permanents {
            goad::goad_for(g, item, permanent, Duration::Indefinite);
        }
    });
    g.end_sentence();
    let mut unchosen = opponents;
    let mut assignments = Vec::with_capacity(permanents.len());
    for &permanent in &permanents {
        // There are never fewer opponents left unchosen than permanents left to give: at most X creature cards were found.
        let opponent = match unchosen.len() {
            1 => unchosen[0],
            _ => {
                // The question names the permanent it is for, so each opponent is chosen knowing what they will get.
                let Some(opponent) = choose::player_about(g, you, item.object, unchosen.clone(), "GainControlOf", permanent)? else { break };
                opponent
            }
        };
        unchosen.retain(|&p| p != opponent);
        assignments.push((permanent, opponent));
    }
    instruction(g, |g| {
        for (permanent, opponent) in assignments {
            g.gain_control(item.object, permanent, opponent, Duration::Indefinite);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        enters,
        "When Dack Fayden enters, reveal cards from the top of your library until you reveal X creature cards, where X is the number of opponents you have. Put those creature cards onto the battlefield, then shuffle. They're goaded for the rest of the game. For each of those permanents, choose a different opponent. Each opponent gains control of the permanent for which they were chosen.",
    )],
    ..oracle::DACK_FAYDEN_HELPING_HAND
};
