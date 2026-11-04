use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, effects, tokens};
use cantrip_oracle as oracle;

/// "For each creature you control, create a 1/2 white Moogle creature token with lifelink. Then creatures you control gain
/// indestructible until end of turn." The number of creatures is counted as the spell resolves, before any token is made, and
/// the tokens are made together as one event (CR 111.1). Then, the creatures the controller has - the new tokens among them -
/// gain indestructible (CR 611.2c).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = creatures_you_control(g, item.controller);
    tokens::create(g, item.controller, &tokens::MOOGLE_1_2_LIFELINK, count as u32)?;
    effects::creatures_you_control_gain(g, item, &[Keyword::Indestructible]);
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&tokens::MOOGLE_1_2_LIFELINK];

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::MOOGLES_VALOR };
