use cantrip_api::*;
use cantrip_mechanics::{damage_target, delayed, library, targets, triggers};
use cantrip_oracle as oracle;

/// "~ deals X damage to target creature. When that creature dies this turn, ..." (CR 603.7c): the delayed triggered ability is
/// fixed on that creature, so it triggers only when that very object dies, whatever killed it (a creature that left and came
/// back is a new object, CR 400.7). It is set up even if the damage is not dealt, as the creature may still die this turn.
fn end_blaze_epiphany(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    damage_target(g, item, 0, item.x as u16);
    delayed::this_turn(g, item, creature);
    Ok(())
}

/// "... exile a number of cards from the top of your library equal to its power, then choose a card exiled this way. Until the
/// end of your next turn, you may play that card." (CR 608.2h, 406): its power as it last was on the battlefield, and the one
/// chosen is playable (a land too, under the ordinary land-play rules, CR 305.1) until the cleanup step of the controller's
/// next turn (CR 514.2); the rest stay exiled. A creature of power 0 or less exiles nothing.
fn exile_and_choose(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = targets::power_or_last_known(g, item.object).max(0) as usize;
    let grant = PlayGrant { instead: None, lands: true, expiry: Expiry::EndOfYourNextTurn, ..PlayGrant::CAST };
    library::exile_top_choose_one_may_play(g, item.controller, item.object, power, grant)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], end_blaze_epiphany),
    delayed: &[triggered(triggers::THIS_DIES, exile_and_choose, "When that creature dies this turn, exile a number of cards from the top of your library equal to its power, then choose a card exiled this way. Until the end of your next turn, you may play that card.")],
    ..oracle::END_BLAZE_EPIPHANY
};
