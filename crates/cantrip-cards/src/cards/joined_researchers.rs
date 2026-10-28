use cantrip_api::*;
use cantrip_mechanics::{cards_in_hand, prepare, targets, triggers};
use cantrip_oracle as oracle;

/// "Secret Rendezvous - You and target opponent each draw three cards." With the target illegal the spell does not
/// resolve (CR 608.2b), so you draw nothing either. Both draw in turn order from the active player (CR 121.2c), who is not
/// always you: a sorcery can be cast on an opponent's turn (Teferi, Time Raveler).
fn secret_rendezvous(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let players: Vec<PlayerId> = g.state().apnap().filter(|&p| p == item.controller || p == opponent).collect();
    for player in players {
        g.draw(player, 3);
    }
    Ok(())
}

/// Joined Researchers' prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static SECRET_RENDEZVOUS: CardDef = CardDef {
    name: "Secret Rendezvous",
    cost: mana("{1}{W}{W}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::OPPONENT], secret_rendezvous),
    text: "You and target opponent each draw three cards.",
    ..CardDef::BLANK
};

/// The intervening "if an opponent has more cards in hand than you" (CR 603.4), checked as the ability triggers and as it
/// resolves.
fn an_opponent_has_more_cards_in_hand(q: &dyn Query, me: &Asker) -> bool {
    let mine = cards_in_hand(q, me.controller);
    q.state().opponents(me.controller).any(|opponent| cards_in_hand(q, opponent) > mine)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_END_STEP,
        prepare::become_prepared,
        "At the beginning of each end step, if an opponent has more cards in hand than you, this creature becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
    )
    .condition(an_opponent_has_more_cards_in_hand)],
    prepare: Some(&SECRET_RENDEZVOUS),
    ..oracle::JOINED_RESEARCHERS
};

pub static EXTRA: &[&CardDef] = &[&SECRET_RENDEZVOUS];
