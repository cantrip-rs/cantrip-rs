use cantrip_api::*;
use cantrip_mechanics::effects::may_return_this_card_to_hand;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep": its owner's.
fn your_upkeep(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::Upkeep } if q.state().turn.active == q.obj(source).owner => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::STEP_BEGAN, your_upkeep), may_return_this_card_to_hand, "At the beginning of your upkeep, you may return this card from your graveyard to your hand.")
        // It returns this card from the graveyard, so it functions only there (CR 113.6m).
        .functions_in(IN_A_GRAVEYARD)],
    ..oracle::SQUEE_GOBLIN_NABOB
};
