use cantrip_api::*;
use cantrip_mechanics::{library, mana, select};
use cantrip_oracle as oracle;

const GATE_CARD: select::Selection = select::Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Gate)]).called("Gate");

/// "Gates with different names you control" (CR 201.2a-b): the largest set of non-overlapping names among the Gates the player
/// controls, each judged on what it is now.
fn gates_with_different_names(q: &dyn Query, player: PlayerId, gate: Subtype) -> usize {
    let names: Vec<Vec<&'static str>> = q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| q.obj(id).controller == player && q.chars(id).has_subtype(gate))
        .map(|id| library::object_names(q, id))
        .collect();
    select::largest_name_set(&names).len()
}

/// "Search your library for a Gate card, put it onto the battlefield, then shuffle. If you control ten or more Gates with
/// different names, you win the game" (CR 701.23, 104.2b): the count is made after the search, with the Gate it found.
fn search_for_a_gate_and_maybe_win(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, GATE_CARD, false)?;
    g.end_sentence();
    let gate = g.words(item.object).subtype(Subtype::Gate);
    if gates_with_different_names(g, item.controller, gate) >= 10 {
        g.win_game(item.controller);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            cost("{3}").and(TAP).and(RETURN_THIS),
            search_for_a_gate_and_maybe_win,
            "{3}, {T}, Return this land to its owner's hand: Search your library for a Gate card, put it onto the battlefield, then shuffle. If you control ten or more Gates with different names, you win the game.",
        ),
    ],
    ..oracle::MAZES_END
};
