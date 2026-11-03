use cantrip_api::*;
use cantrip_mechanics::{choose, library, select, top_of_library};
use cantrip_oracle as oracle;

/// Look at five, put one in hand, then order the remaining cards on top (CR 609.3, 401.4).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let looked = top_of_library(g, player, 5);
    g.look_at(player, &looked);
    let chosen = choose::select(g, player, item.object, Purpose::PutIntoHand, looked.clone(),
        select::Selection::of(select::Count::AsManyAsCan(1), &[]))?;
    g.return_to_hand(&chosen);
    let remaining: Vec<_> = looked.into_iter()
        .filter(|&id| is_live(g, id) && g.obj(id).zone == Zone::library(player))
        .collect();
    let order = library::ask_library_order(g, player, item.object, &remaining, DestinationKind::LibraryTop)?;
    g.arrange_library(player, &order, &[]);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::DIABOLIC_VISION };
