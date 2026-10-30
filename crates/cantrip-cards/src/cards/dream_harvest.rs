use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Each opponent exiles cards from the top of their library until they have exiled cards with total mana value 5 or
/// greater this way. Until end of turn, you may cast cards exiled this way without paying their mana costs" (CR 608.2c,
/// 118.9): each opponent in turn order exiles one card at a time and adds up the mana values of those that really reached
/// exile (a replacement may send one elsewhere, and it is not counted), stopping at 5 or when the library is empty. You may
/// cast any of them, from any opponent, for free until the cleanup step (CR 514.2); a land among them cannot be cast.
fn exile_and_cast_free(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let mut exiled = Vec::new();
    for opponent in opponents {
        let mut total = 0;
        while total < 5 {
            let Some(&top) = g.state().zone(Zone::library(opponent)).first() else { break };
            let moved = g.exile(top);
            if g.obj(moved).zone.is(ZoneKind::Exile) {
                total += g.mana_value(moved);
                exiled.push(moved);
            }
        }
    }
    let grant = PlayGrant { instead: Some(ManaCost::ZERO), expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    for card in exiled {
        g.allow_play(card, item.controller, grant);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], exile_and_cast_free),
    ..oracle::DREAM_HARVEST
};
