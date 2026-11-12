//! "A historic permanent card" (CR 700.6, 110.4a) is a card with a permanent type, so a legendary instant or sorcery card is
//! not one: Jhoira, Weatherlight Corsair's reveal passes it by and puts it on the bottom with the rest. The printed cards
//! that are legendary and not permanents (Urza's Ruinous Blast, Kamahl's Druidic Vow) have no content file, so the
//! scenario is played on a table with a test-only legendary sorcery (`cantrip_script::run_on`).

use std::sync::OnceLock;

use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

/// "Probe Legendary Sorcery": a legendary sorcery card with nothing on it.
static LEGENDARY_SORCERY: CardDef = CardDef {
    name: "Probe Legendary Sorcery",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&LEGENDARY_SORCERY])))
}

/// Bot2 reveals the legendary sorcery card, then Mind Stone, a {2} artifact: only Mind Stone is a historic permanent card,
/// so Bot1 puts it onto the battlefield and loses 2, and the sorcery card is the one revealed card left, put on the bottom
/// of Bot2's library. Were the sorcery counted, the reveal would stop at it and Bot1 would take no Mind Stone.
#[test]
fn a_legendary_sorcery_card_is_not_a_historic_permanent_card() {
    let name = "jhoira-passes-a-legendary-sorcery-card";
    let script = GameScript::parse(
        "
[Bot1 Deck]
Black Lotus
Mox Jet
Mox Pearl
Mox Sapphire
Mox Ruby
Jhoira, Weatherlight Corsair
Island
Island x13

[Bot2 Deck]
Island x7
Probe Legendary Sorcery
Mind Stone
Mountain
Plains
Island x9

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Jhoira, Weatherlight Corsair.Cast
[T1 Bot1.Main2]

[Expect]
Turn = 1
Bot1.Life = 18
Bot1.Battlefield = Jhoira, Weatherlight Corsair; Mind Stone; Mox Jet; Mox Pearl; Mox Sapphire
Bot2.Battlefield.Count = 0
Bot2.Library = Mountain; Plains; Island; Island; Island; Island; Island; Island; Island; Island; Island; Probe Legendary Sorcery
",
        name,
    )
    .unwrap_or_else(|e| panic!("{name}: {e}"));
    if let Err(failure) = run_on(&script, table()) {
        panic!("{name}:\n{}", failure.report);
    }
}
