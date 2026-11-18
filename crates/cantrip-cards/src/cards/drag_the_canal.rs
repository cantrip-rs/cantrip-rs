use cantrip_api::*;
use cantrip_mechanics::{effects, history, tokens};
use cantrip_oracle as oracle;

/// "Create a 2/2 white and blue Detective creature token. If a creature died this turn, you gain 2 life, surveil 2, then investigate."
/// The steps happen in the order written (CR 608.2c). Any creature's death this turn counts (CR 700.4), a token's included.
fn create_a_detective_then_if_a_creature_died(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_a_white_blue_detective(g, item)?;
    if history::a_creature_died_this_turn(g, &Asker::new(item.object, item.controller)) {
        effects::gain_life(g, item, 2)?;
        effects::surveil(g, item, 2)?;
        tokens::investigate(g, item.controller)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], create_a_detective_then_if_a_creature_died), ..oracle::DRAG_THE_CANAL };
pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLUE_DETECTIVE];
