use cantrip_api::*;
use cantrip_mechanics::{library, top_of_library};
use cantrip_oracle as oracle;

/// "{R}, {T}: Look at the top eight cards of your library. Exile four of them at random, then put the rest on top of your library in any order." (CR 608.2, 406.3).
fn look_top_eight_cards_library(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let looked = top_of_library(g, item.controller, 8);
    g.look_at(item.controller, &looked);
    let mut pool = looked.clone();
    let mut picked = Vec::new();
    while picked.len() < 4 {
        let Some(card) = library::random_of(g, &pool) else { break };
        picked.push(card);
        pool.retain(|&id| id != card);
    }
    g.exile_all(&picked);
    let remaining: Vec<ObjectId> = looked.into_iter()
        .filter(|&id| is_live(g, id) && g.obj(id).zone == Zone::library(item.controller))
        .collect();
    let order = library::ask_library_order(
        g,
        item.controller,
        item.object,
        &remaining,
        DestinationKind::LibraryTop,
    )?;
    g.arrange_library(item.controller, &order, &[]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{R}").and(TAP),
            look_top_eight_cards_library,
            "{R}, {T}: Look at the top eight cards of your library. Exile four of them at random, then put the rest on top of your library in any order.",
        ),
    ],
    ..oracle::ORCISH_LIBRARIAN
};
