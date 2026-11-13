use cantrip_api::*;
use cantrip_mechanics::{delayed, triggers};
use cantrip_oracle as oracle;

/// "Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at
/// the beginning of the next end step. Sacrifice the creature when you lose control of this creature." (CR 603.7c, 400.7e,
/// 730.3c): every card the creature became, put onto the battlefield together (CR 101.4) - one that has left the graveyard is a
/// new object the move does not find (CR 400.7) - and each that arrived watched for the sacrifice.
fn put_onto_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<(ObjectId, PlayerId, bool)> = g.event_objects(item).iter().map(|&card| (card, item.controller, false)).collect();
    let returned = g.put_all_onto_battlefield_as(&cards)?;
    g.end_sentence();
    g.delayed_trigger_about(DelayedWhen::WhenEvent, item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), &returned);
    Ok(())
}

/// "... put that card onto the battlefield under your control at the beginning of the next end step." (CR 603.7c, 400.7e):
/// "that card" is what the creature became in the graveyard - every card of a merged permanent (CR 730.3c); a token has ceased
/// to exist and is none.
fn at_next_end_step(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = item.subject.object().map(|old| g.found_all_after_move(old)).unwrap_or_default();
    delayed::at_next_end_step_about(g, item, 1, &cards);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::CREATURE_DEALT_DAMAGE_BY_THIS_DIES,
            at_next_end_step,
            "Whenever a creature dealt damage by this creature this turn dies, put that card onto the battlefield under your control at the beginning of the next end step. Sacrifice the creature when you lose control of this creature.",
        ),
    ],
    delayed: &[
        triggered(
            triggers::YOU_LOSE_CONTROL_OF_THIS,
            delayed::sacrifice_created,
            "Sacrifice the creature when you lose control of this creature.",
        ),
        delayed_trigger(
            put_onto_battlefield,
            "Put that card onto the battlefield under your control at the beginning of the next end step.",
        ),
    ],
    ..oracle::SERAPH
};
