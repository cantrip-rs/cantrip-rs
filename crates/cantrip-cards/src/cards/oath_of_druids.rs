use cantrip_api::*;
use cantrip_mechanics::{effects, library, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("RevealUntilCreature", "reveal until you reveal a creature card?", "It goes onto the battlefield; every other card revealed goes into your graveyard", "Reveal", "Don't reveal");

/// "At the beginning of each player's upkeep" (CR 503.1): the ability is about that player, "the first player".
fn each_upkeep_about_the_active_player(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    if triggers::each_upkeep(q, source, event).is_none() {
        return None;
    }
    Fired::about(Target::Player(q.state().turn.active))
}

/// "Target player who controls more creatures than they do and is their opponent": measured against the player whose
/// upkeep it is, not against the enchantment's controller.
fn player_ahead_on_creatures(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let Target::Player(other) = t else { return false };
    let them = q.state().turn.active;
    other != them && q.state().opponents(them).any(|p| p == other) && cantrip_mechanics::creatures_you_control(q, other) > cantrip_mechanics::creatures_you_control(q, them)
}

/// The first player may reveal cards from the top of their library until they reveal a creature card. If they do, they put
/// that card onto the battlefield and all other cards revealed this way into their graveyard. The whole reveal is worked out
/// before anything moves; a library with no creature card is revealed to the last card and every one of them is buried.
fn reveal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(who) = item.subject else { return Ok(()) };
    if !effects::ask_may(g, who, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let (found, buried) = library::reveal_until(g, who, 1, &|q: &dyn Query, card: ObjectId| {
        q.chars(card).is(CardType::Creature)
    });
    if buried.is_empty() && found.is_empty() {
        return Ok(());
    }
    if !buried.is_empty() {
        g.put_into_graveyard(&buried);
    }
    if let Some(&card) = found.first() {
        g.put_onto_battlefield(card, who, false)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::STEP_BEGAN, each_upkeep_about_the_active_player), reveal, "At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard.")
        .targets(&[target(player_ahead_on_creatures)])
        .subject_chooses_targets()],
    ..oracle::OATH_OF_DRUIDS
};
