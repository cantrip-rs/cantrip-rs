use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature can't be blocked this turn. Draw a card." (CR 608.2c): the target first, then the draw.
fn enter_the_enigma(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_cant_be_blocked_this_turn(g, item)?;
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], enter_the_enigma), ..oracle::ENTER_THE_ENIGMA };
