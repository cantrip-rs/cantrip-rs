use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Exile up to one other target creature you control. Return that card to the battlefield under its owner's control at the
/// beginning of the next end step" (CR 603.7): a delayed ability about the card exiled. A token exiled this way ceases to
/// exist, so the card the ability finds is gone (CR 111.7, 400.7).
fn exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(creature);
    delayed::at_next_end_step(g, item, 0, Target::Object(exiled));
    Ok(())
}

/// The intervening "if one or more cards were put into exile this turn" (CR 603.4): asked of the history, by anyone, from any
/// zone. A token is not a card (CR 108.2, 111.1). Checked as the end step begins and again as the ability resolves.
fn a_card_was_exiled_this_turn(q: &dyn Query, _: &Asker) -> bool {
    q.history()
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).to(ZoneKind::Exile).this_turn())
        .any(|r| matches!(r.event, Event::ZoneChange { new, .. } if !q.obj(new).token))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            exile,
            "When Ennis enters, exile up to one other target creature you control. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL.optional()]),
        triggered(
            YOUR_END_STEP,
            effects::plus_one_counter_on_this,
            "At the beginning of your end step, if one or more cards were put into exile this turn, put a +1/+1 counter on Ennis.",
        )
        .condition(a_card_was_exiled_this_turn),
    ],
    delayed: &[delayed_trigger(delayed::return_subject_to_battlefield, "Return the card exiled with Ennis to the battlefield under its owner's control.")],
    ..oracle::ENNIS_DEBATE_MODERATOR
};
