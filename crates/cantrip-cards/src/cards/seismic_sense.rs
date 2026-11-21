use cantrip_api::*;
use cantrip_mechanics::library::{self, Rest};
use cantrip_mechanics::lands_you_control;
use cantrip_oracle as oracle;

/// "Look at the top X cards of your library, where X is the number of lands you control. You may reveal a creature or land card
/// from among them and put it into your hand. Put the rest on the bottom of your library in a random order." (CR 701.20a,
/// 609.3): X is counted as the spell resolves, from the lands its controller controls; a library with fewer cards shows what it
/// has, and nobody is asked to take a card when none of them is a creature or land.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = lands_you_control(g, item.controller);
    library::look_at_top(g, item.controller, item.object, x, (0, 1), true, |chars| chars.is(CardType::Creature) || chars.is(CardType::Land), Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::SEISMIC_SENSE };
