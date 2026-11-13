use cantrip_api::*;
use cantrip_mechanics::{library, select, start_your_engines};
use cantrip_oracle as oracle;

/// "Search your library for up to X basic land cards, where X is your speed. Put those cards onto the battlefield tapped,
/// then shuffle." (CR 701.23, 702.179): X is the controller's speed as it resolves (a player with no speed has 0, CR
/// 702.179f); the library is searched and shuffled even when X is 0.
fn search_for_speed_basic_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = u32::from(start_your_engines::speed(g.state(), item.controller));
    library::find_to_battlefield(g, item.controller, item.object, select::basic_land_cards(select::up_to(x)), true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[ability(
        cost("{3}{G}").and(SACRIFICE_THIS),
        search_for_speed_basic_lands,
        "{3}{G}, Sacrifice this enchantment: Search your library for up to X basic land cards, where X is your speed. Put those cards onto the battlefield tapped, then shuffle.",
    )],
    ..oracle::POINT_THE_WAY
};
