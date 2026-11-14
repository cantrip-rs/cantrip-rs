use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Exile", "exile the blocked creature and play it from exile this turn?", "", "Exile it", "Leave it");

/// "Whenever a creature you control becomes blocked" (CR 509.1h, 603.2): the creature, one of yours, is the subject; once per
/// creature that becomes blocked, however many creatures block it.
fn creature_you_control_becomes_blocked(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::BecameBlocked { attacker } = event else { return None };
    if q.controller(*attacker) == asker.controller && q.chars(*attacker).is(CardType::Creature) {
        return Fired::about(Target::Object(*attacker));
    }
    None
}

/// "You may exile it. You may play that card from exile this turn" (CR 611.2a): asked only while it is still the creature
/// that became blocked (CR 400.7). Exiling it removes it from combat (CR 506.4), so it deals and takes no combat damage. A token
/// that is exiled ceases to exist (CR 111.7) and is no card to play; a land card may be played as the turn's land drop (CR 305.1).
fn exile_it_and_play_it_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(it) = item.subject else { return Ok(()) };
    if !g.is_on_battlefield(it) || !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let token = g.obj(it).token;
    let card = g.exile(it);
    if !token {
        g.allow_play(card, item.controller, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
    }
    Ok(())
}

/// "Norin can't block." (CR 604.1).
pub static CARD: CardDef = CardDef {
    keywords: Keywords::of(&[Keyword::CantBlock]),
    abilities: &[triggered(
        when_asked(EventMask::BECAME_BLOCKED, creature_you_control_becomes_blocked),
        exile_it_and_play_it_this_turn,
        "Whenever a creature you control becomes blocked, you may exile it. You may play that card from exile this turn.",
    )],
    ..oracle::NORIN_SWIFT_SURVIVALIST
};
