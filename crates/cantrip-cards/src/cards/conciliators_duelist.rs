use cantrip_api::*;
use cantrip_mechanics::triggers::{REPARTEE, THIS_ENTERS};
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Exile up to one target creature. Return that card to the battlefield under its owner's control at the beginning of the next
/// end step" (CR 603.7): a delayed ability about the card exiled - each card, for a merged permanent (CR 730.3c) - which comes
/// back as a new object (CR 400.7). A token exiled this way ceases to exist, so the card the ability finds is gone (CR 111.7).
fn exile_until_end_step(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::exile_target_then_return_at_next_end_step(g, item, 0, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::draw_then_each_player_loses_life(g, item, 1, 1), "When this creature enters, draw a card. Each player loses 1 life."),
        triggered(
            REPARTEE,
            exile_until_end_step,
            "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, exile up to one target creature. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .targets(&[targets::CREATURE.optional()]),
    ],
    delayed: &[delayed_trigger(delayed::return_subject_to_battlefield, "Return that card to the battlefield under its owner's control at the beginning of the next end step.")],
    ..oracle::CONCILIATORS_DUELIST
};
