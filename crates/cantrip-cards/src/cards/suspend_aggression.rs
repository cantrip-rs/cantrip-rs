use cantrip_api::*;
use cantrip_mechanics::{targets, top_of_library};
use cantrip_oracle as oracle;

/// "Exile target nonland permanent and the top card of your library. For each of those cards, its owner may play it until the end
/// of their next turn" (CR 406.2, 601.3): both leave together, and each that really reached exile and is a card (a token ceases to
/// exist, CR 111.7) can be played by its owner, a land included, until the end of that player's own next turn. With the target
/// illegal the spell does nothing (CR 608.2b), so the library card stays.
fn exile_and_let_owners_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let mut exiling = vec![permanent];
    exiling.extend(top_of_library(g, item.controller, 1));
    for exiled in g.exile_all(&exiling) {
        if g.obj(exiled).zone.is(ZoneKind::Exile) && !g.obj(exiled).token {
            let owner = g.obj(exiled).owner;
            g.allow_play(exiled, owner, PlayGrant { instead: None, lands: true, expiry: Expiry::EndOfYourNextTurn, ..PlayGrant::CAST });
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::NONLAND_PERMANENT], exile_and_let_owners_play), ..oracle::SUSPEND_AGGRESSION };
