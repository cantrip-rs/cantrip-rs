use cantrip_api::*;
use cantrip_mechanics::library::exile_top_may_play;
use cantrip_mechanics::tokens::{self, create_treasure};
use cantrip_oracle as oracle;

/// "Exile the top two cards of your library. You may play them this turn (CR 305.1, 601.3: a land too). If you exiled a
/// land card this way, create a 3/1 red Dinosaur creature token. If you exiled a nonland card this way, create a Treasure
/// token." The Dinosaur comes first (the ruling); both are judged by the cards that really reached exile (CR 406.3).
fn exile_and_make(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let grant = PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST };
    let exiled = exile_top_may_play(g, item.controller, 2, item.controller, grant);
    if exiled.iter().any(|&card| g.chars(card).is(CardType::Land)) {
        tokens::create(g, item.controller, &tokens::DINOSAUR, 1)?;
    }
    if exiled.iter().any(|&card| !g.chars(card).is(CardType::Land)) {
        create_treasure(g, item.controller)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(cantrip_mechanics::triggers::YOUR_UPKEEP, exile_and_make, "At the beginning of your upkeep, exile the top two cards of your library. You may play them this turn. If you exiled a land card this way, create a 3/1 red Dinosaur creature token. If you exiled a nonland card this way, create a Treasure token.")],
    ..oracle::BONEHOARD_DRACOSAUR
};

pub static EXTRA: &[&CardDef] = &[&tokens::DINOSAUR];
