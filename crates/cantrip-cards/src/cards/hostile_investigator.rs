use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more players discard one or more cards" (CR 603.2c): any player's discard, once however many cards and
/// players are in the batch; the subject is no one in particular.
fn a_player_discards(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    if matches!(event, Event::Discarded { .. }) { Fired::PLAIN } else { None }
}

/// "Investigate" (CR 701.16a): its controller investigates once as the ability resolves.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

/// The enters ability discards an opponent's card (CR 701.9) and the investigate ability watches every discard, its own
/// controller's included, once each turn (CR 603.2c; no numbered rule for "only once each turn").
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::target_player_discards::<1>, "When this creature enters, target opponent discards a card.")
            .targets(&[targets::OPPONENT]),
        triggered(
            when(EventMask::DISCARDED, a_player_discards),
            investigate,
            "Whenever one or more players discard one or more cards, investigate. This ability triggers only once each turn.",
        )
        .once_per_batch()
        .triggers_each_turn(1),
    ],
    ..oracle::HOSTILE_INVESTIGATOR
};
