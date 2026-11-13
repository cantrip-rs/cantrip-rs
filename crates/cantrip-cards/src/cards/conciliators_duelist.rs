use cantrip_api::*;
use cantrip_mechanics::triggers::{REPARTEE, THIS_ENTERS};
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Draw a card. Each player loses 1 life." (CR 121.1, 119.3): the controller draws first; then every player still in the game
/// loses 1 life, together (CR 101.4, 603.2c).
fn draw_then_each_player_loses_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.end_sentence();
    let losses: Vec<(PlayerId, i32)> = g.state().apnap().map(|p| (p, 1)).collect();
    g.lose_life_together(&losses);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // Repartee: "Exile up to one target creature. Return that card to the battlefield under its owner's control at the
    // beginning of the next end step" (CR 603.7): a delayed ability about the card exiled - each card, for a merged
    // permanent (CR 730.3c) - which comes back as a new object (CR 400.7). A token exiled this way ceases to exist, so the
    // card the ability finds is gone (CR 111.7). The shared slot-0 body.
    abilities: &[
        triggered(THIS_ENTERS, draw_then_each_player_loses_1, "When this creature enters, draw a card. Each player loses 1 life."),
        triggered(
            REPARTEE,
            delayed::exile_target_then_return_at_next_end_step_0,
            "Repartee \u{2014} Whenever you cast an instant or sorcery spell that targets a creature, exile up to one target creature. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .targets(&[targets::CREATURE.optional()]),
    ],
    delayed: &[delayed_trigger(delayed::return_subject_to_battlefield, "Return that card to the battlefield under its owner's control at the beginning of the next end step.")],
    ..oracle::CONCILIATORS_DUELIST
};
