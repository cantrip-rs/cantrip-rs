use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::history::done_this_turn;
use cantrip_mechanics::{connive, cost_reduction, lands, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Connive", "have it connive?", "Draw a card, then discard a card  ·  only once each turn", "Connive", "Don't");

/// "Whenever another Villain you control enters" (CR 603.2, 603.6a), not at all once its controller has done the "do this only
/// once each turn" (CR 603.2h), as Terrasymbiosis does.
fn another_villain_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if done_this_turn(q, me) {
        return None;
    }
    triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Villain) }>(q, me, event)
}

/// "You may have it connive. Do this only once each turn." (CR 701.50): the Villain that entered, if it is still there to
/// take the counter. The limit is spent by the doing, not by the triggering: a "no" leaves the turn unspent, and a triggering
/// that waited while another was done finds it spent.
fn may_connive(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(villain) = item.subject else { return Ok(()) };
    if g.history().done(item.object, item.ability, item.controller) {
        return Ok(());
    }
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.note_done(item.object, item.ability, item.controller);
        connive(g, villain, item.object)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[cost_reduction::subtype_spells_you_cast::<1, { lands::sub(Subtype::Villain) }>],
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_villain_enters),
        may_connive,
        "Whenever another Villain you control enters, you may have it connive. Do this only once each turn.",
    )],
    ..oracle::BARON_STRUCKER_HYDRA_OVERLORD
};
