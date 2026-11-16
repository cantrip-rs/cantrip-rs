use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to create a token copy of {subject} at the beginning of the next end step?", "", "Pay {cost}", "Don't pay");

/// "Whenever you sacrifice this artifact or another artifact" (CR 701.21a, 603.10a): an artifact of yours a sacrifice moved from
/// the battlefield, this one included, read as it was when it left; the trigger is about it.
fn you_sacrifice_an_artifact(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let sacrificed = triggers::sacrificed_by(q, me.controller, event)?;
    if q.chars(sacrificed).is(CardType::Artifact) { Fired::about(Target::Object(sacrificed)) } else { None }
}

/// "You may pay {2}. If you do, at the beginning of the next end step, create a token that's a copy of that artifact" (CR 118.12,
/// 603.7): the payment is chosen as the ability resolves, and the delayed ability is about the artifact that was sacrificed.
fn pay_to_copy_it_at_end_step(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(artifact) = item.subject.object() else { return Ok(()) };
    if g.may_pay(item.controller, item.object, artifact, cost("{2}"), &PAY)? {
        delayed::at_next_end_step_about(g, item, 0, &[artifact]);
    }
    Ok(())
}

/// "Create a token that's a copy of that artifact" (CR 707.2, 111.1): its copiable values as it last was on the battlefield.
fn create_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for original in g.event_objects(item).to_vec() {
        g.create_token_copies(original, item.controller, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, you_sacrifice_an_artifact),
            pay_to_copy_it_at_end_step,
            "Whenever you sacrifice this artifact or another artifact, you may pay {2}. If you do, at the beginning of the next end step, create a token that's a copy of that artifact.",
        )
        .on_sacrifice(),
        ability(cost("{2}").and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 1), "{2}, Sacrifice this artifact: Draw a card."),
    ],
    delayed: &[delayed_trigger(create_copy, "If you do, at the beginning of the next end step, create a token that's a copy of that artifact.")],
    ..oracle::ESOTERIC_DUPLICATOR
};
