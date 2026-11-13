use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// Starting with the controller, each player may put one permanent card down; repeat if one entered (CR 608.2c).
fn put_permanents(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<_> = g.state().turn_order_from(item.controller).collect();
    loop {
        let mut entered = false;
        for &player in &players {
            let cards = g.state().zone(Zone::hand(player)).iter().copied()
                .filter(|&id| !g.obj(id).token && !g.obj(id).card_copy)
                .collect();
            let chosen = choose::select(
                g, player, item.object, Purpose::PutOntoBattlefield, cards,
                select::Selection::of(select::up_to(1), &[select::PERMANENT_CARD]).called("permanent"),
            )?;
            if let Some(&card) = chosen.first() {
                let moved = g.put_onto_battlefield_by(card, player, false, player)?;
                entered |= g.obj(moved).zone.is(ZoneKind::Battlefield);
            }
        }
        g.end_sentence();
        if !entered { break; }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], put_permanents),
    ..oracle::EUREKA
};
