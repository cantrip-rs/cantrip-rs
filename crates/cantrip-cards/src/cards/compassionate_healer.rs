use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "You gain 1 life and scry 1." (CR 119.1, 701.22.)
fn life_and_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 1)?;
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_BECOMES_TAPPED, life_and_scry, "Whenever this creature becomes tapped, you gain 1 life and scry 1.")],
    ..oracle::COMPASSIONATE_HEALER
};
