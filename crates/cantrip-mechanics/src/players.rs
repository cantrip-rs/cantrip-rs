//! "Each player" and "each opponent" steps (CR 101.4): the players are listed in turn order as the step begins, and each is
//! reached in turn. A player who has left the game by the time the step reaches them - a concession while an earlier player
//! was choosing, or as they were about to be asked - is no longer one of "each player" (CR 800.4a): they choose nothing and
//! no one chooses for them. (CR 800.4g, another player making the choice, is for a choice an object asks of a particular
//! player, such as "target opponent chooses".)

use cantrip_api::*;

/// Whether an "each player" step that has come to `player` goes on with them: true while they are still in the game.
pub fn reached(g: &mut dyn Game, player: PlayerId) -> bool {
    g.still_in_game(player)
}

/// The choices of a player the ability of `each_opponent_loses_life_unless_sacrifices_or_discards` asks, in the order of the
/// card's `labels`: "Sacrifice a nonland permanent", "Discard a card", "Lose N life".
const SACRIFICE: u8 = 0;
const DISCARD: u8 = 1;
const LOSE: u8 = 2;

/// "Each opponent loses `life` life unless that player sacrifices a nonland permanent of their choice or discards a card", done
/// `times` over (Rottenmouth Viper's "for each blight counter on it"; Perforating Artist's once). `card` is the card whose
/// `labels` are "Sacrifice a nonland permanent", "Discard a card" and "Lose N life", in that order.
///
/// Each opponent in turn order (CR 101.4) makes all of their choices - one for each time: to sacrifice a nonland permanent,
/// to discard a card, or to lose the life, a permanent or card already chosen not chosen again, and losing the life the one
/// choice left to a player with neither - and then all the actions happen together (the Viper's rulings, CR 101.4): every
/// sacrifice, every discard and every loss of life at once, a player losing life "the appropriate number of times" - one
/// loss for each time it was chosen, each its own occurrence (`Game::together`).
pub fn each_opponent_loses_life_unless_sacrifices_or_discards(g: &mut dyn Game, item: &StackItem, card: &'static CardDef, times: u32, life: i32) -> Result<(), Suspend> {
    let card = g.card_id(card);
    let (mut sacrificed, mut discarded, mut losses) = (Vec::new(), Vec::new(), Vec::new());
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| g.state().opponents(item.controller).any(|o| o == p)).collect();
    for opponent in opponents {
        let asker = g.item_asker(item, opponent);
        for _ in 0..times {
            if !reached(g, opponent) {
                break;
            }
            let permanents: Vec<ObjectId> = g
                .permanents_of(opponent)
                .into_iter()
                .filter(|&id| crate::targets::filters::nonland(g, &asker, id) && !sacrificed.iter().any(|&(_, s)| s == id) && g.can_sacrifice(opponent, id))
                .collect();
            let hand = g.state().zone(Zone::hand(opponent)).to_vec();
            let cards: Vec<ObjectId> = hand.iter().copied().filter(|c| !discarded.contains(c)).collect();
            let mut options = Vec::new();
            if !permanents.is_empty() {
                options.push(SACRIFICE);
            }
            if !cards.is_empty() {
                options.push(DISCARD);
            }
            options.push(LOSE);
            let choice = if options.len() == 1 {
                LOSE
            } else {
                let labels = options.iter().map(|&i| Label::Card(card, i)).collect();
                options[crate::choose::one_of(g, opponent, item.object, labels)? as usize]
            };
            match choice {
                SACRIFICE => sacrificed.extend(crate::choose::cards(g, opponent, item.object, Purpose::Sacrifice, permanents, 1, false)?.into_iter().map(|id| (opponent, id))),
                DISCARD => discarded.extend(crate::choose::cards_from(g, opponent, item.object, Purpose::Discard, hand, cards, 1, false)?),
                _ => losses.push((opponent, -life)),
            }
        }
    }
    g.together(Together { sacrifices: &sacrificed, discards: &discarded, life: &losses, ..Together::default() });
    Ok(())
}
