use cantrip_api::*;
use cantrip_mechanics::{lands, library, mana, select};
use cantrip_oracle as oracle;

const GATE_CARD: select::Selection = select::Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Gate)]).called("Gate");

/// "Search your library for a Gate card, put it onto the battlefield, then shuffle. If you control ten or more Gates with
/// different names, you win the game" (CR 701.23, 104.2b): the count is made after the search, with the Gate it found.
fn search_for_a_gate_and_maybe_win(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, GATE_CARD, false)?;
    g.end_sentence();
    let gate = g.words(item.object).subtype(Subtype::Gate);
    if lands::differently_named_lands_matching(g, item.controller, &|chars| chars.has_subtype(gate)) >= 10 {
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
