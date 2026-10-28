use cantrip_api::*;
use cantrip_oracle as oracle;

/// How many times `creature` has become tapped this turn, as the history stands (CR 603.4's "first time that creature has
/// become tapped this turn" is asked of it).
fn times_tapped_this_turn(q: &dyn Query, creature: ObjectId) -> usize {
    q.history().count(EventQuery::kind(EventMask::TAPPED).this_turn().object(creature))
}

/// "Whenever a creature you control becomes tapped during your turn, if it's the first time that creature has become tapped
/// this turn": the active player is this ability's controller, and the creature is theirs. A trigger sees the history as it
/// was just before its event, so the first time is when no tap is recorded yet.
fn creature_you_control_becomes_tapped_first_time(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Tapped { permanent }
            if q.state().turn.active == me.controller
                && q.chars(*permanent).is(CardType::Creature)
                && q.controller(*permanent) == me.controller
                && times_tapped_this_turn(q, *permanent) == 0 =>
        {
            Fired::about(Target::Object(*permanent))
        }
        _ => None,
    }
}

/// The intervening "if" is checked again as the ability resolves (CR 603.4): the tap that triggered it is the only one this
/// turn.
fn untap_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        if times_tapped_this_turn(g, creature) == 1 {
            g.untap(creature);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::TAPPED, creature_you_control_becomes_tapped_first_time),
        untap_it,
        "Whenever a creature you control becomes tapped during your turn, if it's the first time that creature has become tapped this turn, untap it.",
    )],
    ..oracle::CAPTAIN_AMERICA_LIVING_LEGEND
};
