use cantrip_api::*;
use cantrip_mechanics::{delayed, ferocious, library};
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_oracle as oracle;

/// "Exile the top X cards of your library, where X is the number of creatures you control with power 4 or greater" (CR 107.3,
/// 406): X is worked out once, as the ability resolves, with Dragonhawk counted if it is still there. "You may play those
/// cards until your next end step": a permission to play, so a land may be the turn's land drop (CR 305.1), ending as the
/// controller's next end step begins (CR 500.4). "At the beginning of your next end step, Dragonhawk deals 2 damage ... for
/// each of those cards that are still exiled": the delayed ability (CR 603.7) is about exactly the cards this resolution
/// exiled (CR 603.7c), and is set up even when there are none.
fn exile_and_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = ferocious::count(g, you);
    let exiled = library::exile_top_play_until_your_next_end_step(g, you, x);
    delayed::at_your_next_end_step_about(g, item, 0, &exiled);
    Ok(())
}

/// Dragonhawk deals 2 damage to each opponent for each of the cards that is still the card it was and still in exile
/// (CR 400.7): one source, one amount for each opponent, dealt together (CR 120.3).
fn burn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let still_exiled = library::still_exiled(g, g.event_objects(item)).len();
    if still_exiled == 0 {
        return Ok(());
    }
    let amount = 2 * still_exiled as u16;
    let hits: Vec<(ObjectId, Target, i64)> = g.state().opponents(item.controller).map(|p| (item.object, Target::Player(p), i64::from(amount))).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_OR_ATTACKS,
        exile_and_play,
        "Whenever Dragonhawk enters or attacks, exile the top X cards of your library, where X is the number of creatures you control with power 4 or greater. You may play those cards until your next end step. At the beginning of your next end step, Dragonhawk deals 2 damage to each opponent for each of those cards that are still exiled.",
    )],
    delayed: &[delayed_trigger(burn, "At the beginning of your next end step, Dragonhawk deals 2 damage to each opponent for each of those cards that are still exiled.")],
    ..oracle::DRAGONHAWK_FATES_TEMPEST
};
