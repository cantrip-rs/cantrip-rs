use cantrip_api::*;
use cantrip_mechanics::{triggers::THIS_ENTERS, vivid};
use cantrip_oracle as oracle;

/// "Vivid - When this creature enters, draw cards equal to the number of colors among permanents you control": counted as the
/// ability resolves (CR 608.2h), with the Shinestriker itself, already on the battlefield, among the permanents.
fn draw_vivid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let colors = vivid::count(g, item.controller);
    g.draw(item.controller, colors as u32);
    Ok(())
}

/// Flying is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        draw_vivid,
        "Vivid \u{2014} When this creature enters, draw cards equal to the number of colors among permanents you control.",
    )],
    ..oracle::SHINESTRIKER
};
