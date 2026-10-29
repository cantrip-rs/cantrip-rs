use cantrip_api::*;
use cantrip_mechanics::history::gained_life_this_turn;
use cantrip_mechanics::library::{self, Rest};
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library. You may reveal a creature or land card from among them and put it into your hand.
/// If you gained life this turn, you may instead reveal two creature and/or land cards from among them and put them into your
/// hand. Put the rest on the bottom of your library in a random order." The infusion is judged as the spell resolves (CR 608.2c):
/// with it, up to two cards (one stays a choice the first sentence allows).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let most = if gained_life_this_turn(g, item.controller) { 2 } else { 1 };
    library::look_at_top(
        g,
        item.controller,
        item.object,
        4,
        (0, most),
        true,
        |chars| chars.is(CardType::Creature) || chars.is(CardType::Land),
        Rest::BottomRandom,
    )?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::FOLLOW_THE_LUMARETS };
