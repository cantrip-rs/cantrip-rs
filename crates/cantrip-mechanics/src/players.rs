//! "Each player" and "each opponent" steps (CR 101.4): the players are listed in turn order as the step begins, and each is
//! reached in turn. A player who has left the game by the time the step reaches them - a concession while an earlier player
//! was choosing, or as they were about to be asked - is no longer one of "each player" (CR 800.4a): they choose nothing and
//! no one chooses for them. (CR 800.4g, another player making the choice, is for a choice an object asks of a particular
//! player, such as "target opponent chooses".)

use cantrip_api::*;
use crate::targets::filters::nonland;

/// Whether an "each player" step that has come to `player` goes on with them: true while they are still in the game.
pub fn reached(g: &mut dyn Game, player: PlayerId) -> bool {
    g.still_in_game(player)
}

/// The choices "[a player] loses N life unless they sacrifice a nonland permanent of their choice or discard a card" asks, in the
/// order of the card's `labels`: "Sacrifice a nonland permanent", "Discard a card", "Lose N life".
const SACRIFICE: u8 = 0;
const DISCARD: u8 = 1;
const LOSE: u8 = 2;

/// "Each opponent loses `life` life unless that player sacrifices a nonland permanent of their choice or discards a card" (or
/// "unless they discard a card or sacrifice a creature"), done `times` over (Rottenmouth Viper's "for each blight counter on
/// it"; Perforating Artist's and Polygraph Orb's once). `sacrifice` is the permanents the sacrifice may be of
/// (`targets::filters::nonland`, `targets::filters::creature`). `card` is the card whose `labels` are the sacrifice, "Discard a
/// card" and "Lose N life", in that order.
///
/// Each opponent in turn order (CR 101.4) makes all of their choices - one for each time: to sacrifice a permanent,
/// to discard a card, or to lose the life, a permanent or card already chosen not chosen again, and losing the life the one
/// choice left to a player with neither - and then all the actions happen together (the Viper's rulings, CR 101.4): every
/// sacrifice, every discard and every loss of life at once, a player losing life "the appropriate number of times" - one
/// loss for each time it was chosen, each its own occurrence (`Game::together`).
pub fn each_opponent_loses_life_unless_sacrifices_or_discards(g: &mut dyn Game, item: &StackItem, card: &'static CardDef, sacrifice: ObjectFilter, times: u32, life: i32) -> Result<(), Suspend> {
    let card = g.card_id(card);
    let mut chosen = Chosen::default();
    let opponents: Vec<PlayerId> = g.state().apnap().filter(|&p| g.state().opponents(item.controller).any(|o| o == p)).collect();
    for opponent in opponents {
        for _ in 0..times {
            if !reached(g, opponent) {
                break;
            }
            loses_life_unless_sacrifices_or_discards(g, item, card, opponent, sacrifice, life, &mut chosen)?;
        }
    }
    chosen.happen(g);
    Ok(())
}

/// "Target opponent [or player] loses `LIFE` life unless they sacrifice a nonland permanent of their choice or discard a card" as an
/// ability's whole effect (CR 608.2d, 119.3, 701.9, 701.21a), its target slot saying which players it may name: the target player
/// chooses once as it resolves to sacrifice a nonland permanent, to discard a card, or to lose the life, and always may lose the
/// life, even holding cards and permanents (the rulings); with neither to give they lose it. The choice is done as one action
/// (`Game::together`). With the target gone the ability does nothing (CR 608.2b). The ability's card's `labels` are "Sacrifice a
/// nonland permanent", "Discard a card" and "Lose N life", in that order. Thornplate Intimidator and Starseer Mentor print it.
pub fn target_player_loses_life_unless_sacrifices_or_discards<const LIFE: i32>(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.target(0).player() else { return Ok(()) };
    let mut chosen = Chosen::default();
    loses_life_unless_sacrifices_or_discards(g, item, item.ability.card, player, nonland, LIFE, &mut chosen)?;
    chosen.happen(g);
    Ok(())
}

/// The sacrifices, discards and losses of life chosen so far, done together once every choice is made (`Game::together`).
#[derive(Default)]
struct Chosen {
    sacrificed: Vec<(PlayerId, ObjectId)>,
    discarded: Vec<ObjectId>,
    losses: Vec<(PlayerId, i32)>,
}

impl Chosen {
    fn happen(&self, g: &mut dyn Game) {
        g.together(Together { sacrifices: &self.sacrificed, discards: &self.discarded, life: &self.losses, ..Together::default() });
    }
}

/// One choice of `player` for "loses `life` life unless they sacrifice a permanent (`sacrifice`) of their choice or discard a card", asked
/// with `card`'s labels: to sacrifice a permanent `sacrifice` allows, to discard a card, or to lose the life, a permanent or card already in
/// `chosen` not offered again, and losing the life the one choice left (asked nothing) to a player with neither.
fn loses_life_unless_sacrifices_or_discards(g: &mut dyn Game, item: &StackItem, card: CardId, player: PlayerId, sacrifice: ObjectFilter, life: i32, chosen: &mut Chosen) -> Result<(), Suspend> {
    let asker = g.item_asker(item, player);
    let permanents: Vec<ObjectId> = g
        .permanents_of(player)
        .into_iter()
        .filter(|&id| sacrifice(g, &asker, id) && !chosen.sacrificed.iter().any(|&(_, s)| s == id) && g.can_sacrifice(player, id))
        .collect();
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let cards: Vec<ObjectId> = hand.iter().copied().filter(|c| !chosen.discarded.contains(c)).collect();
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
        options[crate::choose::one_of(g, player, item.object, labels)? as usize]
    };
    match choice {
        SACRIFICE => chosen.sacrificed.extend(crate::choose::cards(g, player, item.object, Purpose::Sacrifice, permanents, 1, false)?.into_iter().map(|id| (player, id))),
        DISCARD => chosen.discarded.extend(crate::choose::cards_from(g, player, item.object, Purpose::Discard, hand, cards, 1, false)?),
        _ => chosen.losses.push((player, -life)),
    }
    Ok(())
}
