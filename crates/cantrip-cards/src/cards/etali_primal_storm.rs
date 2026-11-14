use cantrip_api::*;
use cantrip_mechanics::{choose, triggers};
use cantrip_oracle as oracle;

/// "Exile the top card of each player's library, then you may cast any number of spells from among those cards without paying
/// their mana costs" (CR 608.2c, 608.2g, 118.9): the top cards leave together; a library with no card exiles nothing, and a
/// card a replacement effect sent elsewhere was not exiled and is not among them. The controller is then offered the castable
/// ones one at a time (`choose::cast_any_number_free`); lands among them are not spells and stay in exile.
fn exile_top_of_each_library_and_cast_free(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tops: Vec<ObjectId> = {
        let q: &dyn Query = g;
        q.state().apnap().filter_map(|p| q.state().zone(Zone::library(p)).first().copied()).collect()
    };
    let exiled: Vec<ObjectId> = g.exile_all(&tops).into_iter().filter(|&c| g.obj(c).zone.is(ZoneKind::Exile)).collect();
    g.end_sentence();
    choose::cast_any_number_free(g, item.controller, item.object, &exiled)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        exile_top_of_each_library_and_cast_free,
        "Whenever Etali attacks, exile the top card of each player's library, then you may cast any number of spells from among those cards without paying their mana costs.",
    )],
    ..oracle::ETALI_PRIMAL_STORM
};
