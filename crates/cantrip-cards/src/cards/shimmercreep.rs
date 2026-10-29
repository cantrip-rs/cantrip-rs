use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::vivid;
use cantrip_oracle as oracle;

/// "Each opponent loses X life and you gain X life, where X is the number of colors among permanents you control" (CR 207.2c,
/// 608.2h): X is worked out once, as the ability resolves, the Shimmercreep itself counted.
fn drain_for_each_color(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = vivid::count(g, item.controller) as i32;
    g.each_opponent_loses_life(item.controller, x);
    g.gain_life(item.controller, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        drain_for_each_color,
        "Vivid - When this creature enters, each opponent loses X life and you gain X life, where X is the number of colors among permanents you control.",
    )],
    ..oracle::SHIMMERCREEP
};
