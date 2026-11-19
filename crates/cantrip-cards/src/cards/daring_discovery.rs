use cantrip_api::*;
use cantrip_mechanics::{discover, effects, targets};
use cantrip_oracle as oracle;

/// "Up to three target creatures can't block this turn. Discover 4." (CR 509.1b, 611.2a, 608.2b, 701.57a). Each creature that is
/// still a legal target as the spell resolves can't block until end of turn - an effect on the creature, not an ability it gains.
/// The two paragraphs are separate instructions: the shared effect ends the first before the discover begins.
fn cant_block_then_discover(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_target_cant_block_this_turn(g, item)?;
    discover::discover(g, item.controller, 4)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE.between(0, 3)], cant_block_then_discover), ..oracle::DARING_DISCOVERY };
