use cantrip_api::*;
use cantrip_mechanics::triggers::{REPARTEE, THIS_ENTERS};
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Draw a card. Each player loses 1 life." (CR 121.1, 119.3): the controller draws first; then every player still in the game
/// loses 1 life, in turn order.
fn draw_then_each_player_loses_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.end_sentence();
    for player in g.state().apnap().collect::<Vec<_>>() {
        g.lose_life(player, 1);
    }
    Ok(())
}

/// "Exile up to one target creature. Return that card to the battlefield under its owner's control at the beginning of the next
/// end step" (CR 603.7): a delayed ability about the card exiled, which comes back as a new object (CR 400.7). A token exiled
/// this way ceases to exist, so the card the ability finds is gone (CR 111.7).
fn exile_until_end_step(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(creature);
    if g.obj(exiled).zone.is(ZoneKind::Exile) {
        delayed::at_next_end_step(g, item, 0, Target::Object(exiled));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, draw_then_each_player_loses_1, "When this creature enters, draw a card. Each player loses 1 life."),
        triggered(
            REPARTEE,
            exile_until_end_step,
            "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, exile up to one target creature. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .targets(&[targets::CREATURE.optional()]),
    ],
    delayed: &[delayed_trigger(delayed::return_subject_to_battlefield, "Return the card exiled with Conciliator's Duelist to the battlefield under its owner's control.")],
    ..oracle::CONCILIATORS_DUELIST
};
