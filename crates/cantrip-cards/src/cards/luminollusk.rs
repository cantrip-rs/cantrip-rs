use cantrip_api::*;
use cantrip_mechanics::{effects, triggers, vivid};
use cantrip_oracle as oracle;

/// "Vivid - When this creature enters, you gain life equal to the number of colors among permanents you control": counted as the
/// ability resolves (CR 608.2h), the Luminollusk itself, already on the battlefield, among the permanents.
fn gain_vivid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let colors = vivid::count(g, item.controller);
    effects::gain_life(g, item, colors as i32)
}

/// Deathtouch is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        gain_vivid,
        "Vivid \u{2014} When this creature enters, you gain life equal to the number of colors among permanents you control.",
    )],
    ..oracle::LUMINOLLUSK
};
