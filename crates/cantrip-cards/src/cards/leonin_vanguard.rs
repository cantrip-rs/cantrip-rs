use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "This creature gets +1/+1 until end of turn and you gain 1 life" (CR 611.2a, 119.3): the pump from the ability's source
/// (nothing once it has left the battlefield, CR 400.7), then, in the same sentence, the life, which is gained either way.
fn gets_and_you_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::this_gets::<1, 1>(g, item)?;
    effects::gain_life(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        gets_and_you_gain_life,
        "At the beginning of combat on your turn, if you control three or more creatures, this creature gets +1/+1 until end of turn and you gain 1 life.",
    )
    .condition(triggers::you_control_three_or_more_creatures)],
    ..oracle::LEONIN_VANGUARD
};
