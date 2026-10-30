use cantrip_api::*;
use cantrip_mechanics::{library::find_to_hand, select::{basic_land_cards, up_to}, triggers::THIS_ENTERS, vivid};
use cantrip_oracle as oracle;

/// "Vivid — When this enchantment enters, search your library for up to X basic land cards, where X is the number of colors
/// among permanents you control. Reveal those cards, put them into your hand, then shuffle" (CR 701.23, 207.2c, 105.2): X is counted as
/// the ability resolves, this enchantment (green) being among the permanents unless it has left the battlefield.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = u32::from(vivid::count(g, item.controller));
    find_to_hand(g, item.controller, item.object, basic_land_cards(up_to(x)), true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        search,
        "Vivid — When this enchantment enters, search your library for up to X basic land cards, where X is the number of colors among permanents you control. Reveal those cards, put them into your hand, then shuffle.",
    )],
    // "You may play an additional land on each of your turns." (CR 305.2: cumulative with other such effects.)
    play_rules: &[PlayRule::ExtraLandPlays(1)],
    ..oracle::PRISMATIC_UNDERCURRENTS
};
