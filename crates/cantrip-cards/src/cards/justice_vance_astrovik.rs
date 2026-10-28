use cantrip_api::*;
use cantrip_mechanics::effects::{plus_one_counter_on_this, return_first_target_to_hand};
use cantrip_mechanics::targets::nonland_nontoken_permanent_filter;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Whenever another nonland permanent you control is returned to its owner's hand" (CR 603.10a): a permanent other than
/// Justice that went from the battlefield to a hand, a nonland permanent under Justice's controller's control as it last was
/// on the battlefield (a token too: it is in the hand for a moment before it ceases to exist, CR 111.7), whoever returned it.
fn another_nonland_permanent_you_control_is_returned_to_hand(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if *old != me.source
                && from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Hand)
                && !q.chars(*old).is(CardType::Land)
                && q.obj(*old).controller == me.controller =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            return_first_target_to_hand,
            "When Justice enters, return up to one target nonland, nontoken permanent to its owner's hand.",
        )
        .targets(&[up_to_one(nonland_nontoken_permanent_filter)]),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, another_nonland_permanent_you_control_is_returned_to_hand),
            plus_one_counter_on_this,
            "Whenever another nonland permanent you control is returned to its owner's hand, put a +1/+1 counter on Justice.",
        ),
    ],
    ..oracle::JUSTICE_VANCE_ASTROVIK
};
