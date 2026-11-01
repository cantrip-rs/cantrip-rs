use cantrip_api::*;
use cantrip_mechanics::{choose, library, select, triggers::YOUR_FIRST_MAIN_PHASE, vivid};
use cantrip_oracle as oracle;

/// "Vivid — At the beginning of your first main phase, reveal cards from the top of your library until you reveal X nonland
/// cards, where X is the number of colors among permanents you control. For each of those colors, you may exile a card of that
/// color from among the revealed cards. Then shuffle. You may cast the exiled cards this turn" (CR 701.20, 207.2c, 105.2,
/// 701.24): the colors, and so X, are worked out once, as the ability resolves; the cards are revealed together down to the
/// Xth nonland card, the whole library if it holds fewer. Each color in turn offers the revealed cards that are that color,
/// none the earlier colors already chose, so a multicolored card is exiled for one color only, and for no color is more than
/// one exiled; the cards chosen are exiled together, one event (CR 608.2c, 101.4: Laelia, the Blade Reforged triggers once).
/// Every revealed card not exiled is in the library, which is shuffled; the exiled cards may be cast this turn,
/// until the cleanup step (CR 514.2), at the timing their types give them (the rulings), and a land among them (no color) is
/// never one of them.
fn reveal_exile_one_of_each_color_and_cast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let colors = vivid::colors(g, you);
    let x = colors.iter().count();
    let (nonland, others) = library::reveal_until_n(g, you, x, &|q, card| !q.chars(card).is(CardType::Land));
    let mut revealed: Vec<ObjectId> = nonland.iter().chain(&others).copied().collect();
    let mut chosen: Vec<ObjectId> = Vec::new();
    for color in colors.iter() {
        let eligible: Vec<ObjectId> = revealed.iter().copied().filter(|&card| g.chars(card).colors.has(color)).collect();
        if eligible.is_empty() {
            continue;
        }
        let rule = Selection::listed(select::up_to(1), &revealed, eligible).called("card of that color");
        for card in choose::select(g, you, item.object, Purpose::Exile, revealed.clone(), rule)? {
            revealed.retain(|&c| c != card);
            chosen.push(card);
        }
    }
    let grant = PlayGrant { expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    for exiled in g.exile_all(&chosen) {
        if g.obj(exiled).zone.is(ZoneKind::Exile) {
            g.allow_play(exiled, you, grant);
        }
    }
    g.shuffle(you);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_FIRST_MAIN_PHASE,
        reveal_exile_one_of_each_color_and_cast,
        "Vivid — At the beginning of your first main phase, reveal cards from the top of your library until you reveal X nonland cards, where X is the number of colors among permanents you control. For each of those colors, you may exile a card of that color from among the revealed cards. Then shuffle. You may cast the exiled cards this turn.",
    )],
    ..oracle::SANAR_INNOVATIVE_FIRST_YEAR
};
