use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// Each player may pay 1 life for each land; the other lands are destroyed (CR 101.4, 118.12a, 701.8a).
fn cleanse_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut unpaid: Vec<_> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    let players: Vec<_> = g.state().apnap().collect();
    let mut payments = Vec::new();
    for player in players {
        let most = g.payable_resolution_life(player);
        if most == 0 || unpaid.is_empty() { continue; }
        let rule = select::cards(select::up_to(most)).called("lands to pay 1 life for");
        let chosen = choose::select(g, player, item.object, Purpose::Choose, unpaid.clone(), rule)?;
        payments.extend(chosen.iter().map(|_| (player, 1)));
        unpaid.retain(|land| !chosen.contains(land));
    }
    g.pay_life_with_permanents(item.object, &payments, &unpaid, &[]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], cleanse_lands),
    ..oracle::CLEANSING
};
