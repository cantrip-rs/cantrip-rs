use cantrip_api::*;
use cantrip_mechanics::top_of_library;
use cantrip_mechanics::{choose, plot, select};
use cantrip_oracle as oracle;

/// "Look at the top three cards of your library. You may exile a nonland card from among them. If you do, it becomes plotted. Put
/// the rest into your hand." (CR 701.20e, 608.2d, 702.170a): a library with fewer cards shows what it has (CR 609.3). The nonland
/// card is chosen as the spell resolves, declining being the empty answer, and with only lands among them the choice is empty. It
/// becomes plotted if it reached exile (CR 702.170d); the rest, the others still in the library, are put into the hand.
fn look_exile_a_nonland_plot_rest_to_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, 3);
    if looked.is_empty() {
        return Ok(());
    }
    g.look_at(you, &looked);
    let chosen = choose::select(g, you, item.object, Purpose::Plot, looked.clone(), select::NONLAND_CARD)?;
    if let Some(&card) = chosen.first() {
        let exiled = g.exile(card);
        plot::become_plotted(g, exiled);
    }
    let rest: Vec<ObjectId> = looked.iter().copied().filter(|&c| g.obj(c).zone == Zone::library(you)).collect();
    g.return_to_hand(&rest);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], look_exile_a_nonland_plot_rest_to_hand), ..oracle::MAKE_YOUR_OWN_LUCK };
