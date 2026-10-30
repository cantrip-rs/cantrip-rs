use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Up to one other target creature or artifact you control" (CR 115.1d, 603.3d).
fn another_creature_or_artifact_you_control(q: &dyn Query, me: &Asker, t: Target) -> bool {
    t != Target::Object(me.source) && (targets::ARTIFACT_OR_CREATURE_YOU_CONTROL.filter)(q, me, t)
}

/// "If a permanent was returned to your hand from the battlefield this turn" (CR 603.4).
/// Read the destination in the history: returning someone else's permanent to their hand does not qualify.
fn permanent_returned_to_your_hand(q: &dyn Query, me: &Asker) -> bool {
    q.history().select(EventQuery::kind(EventMask::ZONE_CHANGE).this_turn().from(ZoneKind::Battlefield).to(ZoneKind::Hand))
        .any(|record| matches!(record.event, Event::ZoneChange { to, .. } if to == Zone::hand(me.controller)))
}

/// "Draw a card" (CR 121.1).
fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::return_first_target_to_hand,
            "When Miles O'Brien enters, return up to one other target creature or artifact you control to its owner's hand.")
            .targets(&[target(another_creature_or_artifact_you_control).optional()]),
        triggered(triggers::EACH_END_STEP, draw,
            "At the beginning of each end step, if a permanent was returned to your hand from the battlefield this turn, draw a card.")
            .condition(permanent_returned_to_your_hand),
    ],
    ..oracle::MILES_OBRIEN_TRANSPORTER_OPS
};
