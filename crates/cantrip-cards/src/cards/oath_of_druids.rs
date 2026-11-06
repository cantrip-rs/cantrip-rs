use cantrip_api::*;
use cantrip_mechanics::triggers::EACH_PLAYERS_UPKEEP;
use cantrip_mechanics::{effects, library};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("RevealUntilCreature", "reveal until you reveal a creature card?", "It goes onto the battlefield; every other card revealed goes into your graveyard", "Reveal", "Don't reveal");

/// "Target player who controls more creatures than they do and is their opponent": measured against the player whose
/// upkeep it is, not against the enchantment's controller.
fn player_ahead_on_creatures(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let Target::Player(other) = t else { return false };
    let them = q.state().turn.active;
    other != them && q.state().opponents(them).any(|p| p == other) && cantrip_mechanics::creatures_you_control(q, other) > cantrip_mechanics::creatures_you_control(q, them)
}

/// The first player may reveal cards from the top of their library until they reveal a creature card. If they do, they put
/// that card onto the battlefield and all other cards revealed this way into their graveyard, in that order (CR 608.2c): a
/// permanent the creature card becomes sees the rest go to the graveyard. A library with no creature card is revealed to the
/// last card and every one of them is buried.
fn reveal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(who) = item.subject else { return Ok(()) };
    if !effects::ask_may(g, who, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let (creature, rest) = library::reveal_until(g, who, &|q, card| q.chars(card).is(CardType::Creature));
    if let Some(card) = creature {
        g.put_onto_battlefield_by(card, who, false, who)?;
    }
    let buried: Vec<(ObjectId, Zone)> = rest.into_iter().map(|c| (c, Zone::graveyard(who))).collect();
    if !buried.is_empty() {
        g.move_together(&buried, MoveReason::Other);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(EACH_PLAYERS_UPKEEP, reveal, "At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard.")
        .targets(&[target(player_ahead_on_creatures)])
        .subject_chooses_targets()],
    ..oracle::OATH_OF_DRUIDS
};
