use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{connive, lands, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Connive", "have it connive?", "Draw a card, then discard a card  ·  only once each turn", "Connive", "Don't");

/// "Villain spells you cast cost {1} less to cast" (CR 601.2f): a spell of yours that is a Villain as it is cast, read through
/// the words of this creature's text (CR 612); generic mana only (CR 118.7a).
fn villain_spells_cost_one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && q.chars(spell).has_subtype(q.words(source).subtype(Subtype::Villain)) {
        CostDelta::less(1)
    } else {
        CostDelta::NONE
    }
}

/// The ability the once-each-turn record is kept under.
fn connive_ability(q: &dyn Query, source: ObjectId) -> AbilityRef {
    AbilityRef { card: q.chars(source).card, index: 0 }
}

/// "Whenever another Villain you control enters" (CR 603.2, 603.6a), not at all once its controller has done the "do this only
/// once each turn" (CR 603.2h), as Terrasymbiosis does.
fn another_villain_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if q.history().done(me.source, connive_ability(q, me.source)) {
        return None;
    }
    triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Villain) }>(q, me, event)
}

/// "You may have it connive. Do this only once each turn." (CR 701.50): the Villain that entered, if it is still there to
/// take the counter. The limit is spent by the doing, not by the triggering: a "no" leaves the turn unspent, and a triggering
/// that waited while another was done finds it spent.
fn may_connive(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(villain) = item.subject else { return Ok(()) };
    if g.history().done(item.object, item.ability) {
        return Ok(());
    }
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.note_done(item.object, item.ability);
        connive(g, villain, item.object)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[villain_spells_cost_one_less],
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_villain_enters),
        may_connive,
        "Whenever another Villain you control enters, you may have it connive. Do this only once each turn.",
    )],
    ..oracle::BARON_STRUCKER_HYDRA_OVERLORD
};
