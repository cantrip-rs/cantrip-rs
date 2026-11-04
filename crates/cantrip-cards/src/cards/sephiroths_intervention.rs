use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature. You gain 2 life." (CR 701.8, 119.1): the life is gained whether or not the creature was destroyed
/// (an indestructible one stays), as long as the spell resolved; if its only target is gone it does not resolve (CR 608.2b)
/// and no life is gained.
fn destroy_and_gain_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_first_target(g, item)?;
    g.gain_life(item.controller, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], destroy_and_gain_2),
    ..oracle::SEPHIROTHS_INTERVENTION
};
