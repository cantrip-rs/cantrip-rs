use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::VALIANT};
use cantrip_oracle as oracle;

/// "It gets +1/+0 and gains first strike until end of turn. Scry 1." Two instructions (CR 608.2c): the pump and the keyword
/// together, then the scry, which happens whether or not the creature is still on the battlefield (CR 400.7).
fn pump_then_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::this_gets_and_gains_until_end_of_turn::<1, 0, { Keywords::of(&[Keyword::FirstStrike]).0 }>(g, item)?;
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        VALIANT,
        pump_then_scry,
        "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, it gets +1/+0 and gains first strike until end of turn. Scry 1.",
    )],
    ..oracle::VETERAN_GUARDMOUSE
};
