use cantrip_api::*;
use cantrip_mechanics::{hand, lands, triggers};
use cantrip_oracle as oracle;

/// "You may put a land card from your hand onto the battlefield. If you do" (CR 608.2c, 608.2d): only an actual entry
/// creates the delayed ability, controlled by the player who controlled this triggered ability as it resolved (CR 603.7e).
fn put_land_and_return_one_later(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(entered) = hand::may_put_onto_battlefield(g, item.controller, item.object, |c| c.is(CardType::Land))? else {
        return Ok(());
    };
    if !g.is_on_battlefield(entered) {
        return Ok(());
    }
    g.end_sentence();
    let card = g.card_id(&CARD);
    g.delayed_trigger(
        DelayedWhen::Step { step: Step::End, on_turn_of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_BECOMES_TAPPED,
        put_land_and_return_one_later,
        "Whenever Erica Ortegas becomes tapped, you may put a land card from your hand onto the battlefield. If you do, return a land you control to its owner's hand at the beginning of the next end step.",
    )],
    delayed: &[delayed_trigger(
        lands::return_a_land,
        "At the beginning of the next end step, return a land you control to its owner's hand.",
    )],
    ..oracle::ERICA_ORTEGAS_PLUCKY_PILOT
};
