use cantrip_api::*;
use cantrip_mechanics::history::opponents_who_lost_life_this_turn;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your end step, investigate for each opponent who lost life this turn" (CR 701.16a, 119.3): one Clue for
/// each opponent whose life lost this turn is above 0 (one who gained and lost life still lost life), counted as the ability
/// resolves. Each is an investigation (CR 701.16a), made one after the other; none when no opponent lost life.
fn investigate_for_each_opponent_who_lost_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for _ in 0..opponents_who_lost_life_this_turn(g, item.controller) {
        tokens::investigate(g, item.controller)?;
    }
    Ok(())
}

/// "Whenever a Clue you control is put into a graveyard from the battlefield" (CR 603.6c, 603.10a): the Clue as it last existed
/// on the battlefield, under the controller it had there. A Clue that a source put into the graveyard from another zone, or
/// that another player controlled, does not trigger it.
fn clue_you_control_put_into_graveyard(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.chars(*old).has_subtype(Subtype::Clue)
                && q.controller(*old) == me.controller =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

fn create_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::WHITE_BLACK_FLYING_SPIRIT, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_END_STEP,
            investigate_for_each_opponent_who_lost_life,
            "At the beginning of your end step, investigate for each opponent who lost life this turn.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, clue_you_control_put_into_graveyard),
            create_spirit,
            "Whenever a Clue you control is put into a graveyard from the battlefield, create a 1/1 white and black Spirit creature token with flying. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1),
    ],
    ..oracle::TEYSA_OPULENT_OLIGARCH
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE, &tokens::WHITE_BLACK_FLYING_SPIRIT];
