use cantrip_api::*;
use cantrip_mechanics::history::you_gained_or_lost_life_this_turn;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_oracle as oracle;

/// "...look at the top four cards of your library. You may reveal a creature card with power 3 or less from among them and put it
/// into your hand. Put the rest on the bottom of your library in a random order." (CR 701.20a, 401.4): a creature card's power
/// is the printed one in the library (CR 208.1); a card with no power, a noncreature card or a bigger creature cannot be taken.
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top(g, item.controller, item.object, 4, (0, 1), true, |c| c.is(CardType::Creature) && c.power.is_some_and(|p| p <= 3), Rest::BottomRandom)?;
    Ok(())
}

/// "At the beginning of your end step, if you gained or lost life this turn" is an intervening "if" (CR 603.4): checked as the step
/// begins and again as the ability resolves, so it does not trigger at all if no life changed by then (the ruling).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        look,
        "At the beginning of your end step, if you gained or lost life this turn, look at the top four cards of your library. You may reveal a creature card with power 3 or less from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
    )
    .condition(you_gained_or_lost_life_this_turn)],
    ..oracle::STAR_CHARTER
};
