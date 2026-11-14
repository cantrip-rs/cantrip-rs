//! A departed ability source uses its last-known blocking relationship (CR 608.2b).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

#[test]
fn ability_resolves_when_source_is_sacrificed_after_activation() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all()));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus x2
Mox Pearl
Sentinel
Ashnod's Altar
Island x20
[Bot2 Deck]
Black Lotus
Grizzly Bears
Island x20
[Actions]
Bot1.Hand.Black Lotus#1.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Black Lotus#1.Activate(Color: White)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Sentinel.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Hand.Ashnod's Altar.Cast
[T2 Bot2.Main1]
Bot2.Hand.Black Lotus.Cast
Bot2.Battlefield.Black Lotus.Activate(Color: Green)
Bot2.Hand.Grizzly Bears.Cast
[T3 Bot1.DeclareAttackers]
Bot1.Attack(Sentinel > Bot2)
Bot2.Block(Grizzly Bears > Sentinel)
Bot1.Battlefield.Sentinel.Activate(Bot2.Battlefield.Grizzly Bears)
Bot1.Battlefield.Ashnod's Altar.Activate(Sacrifice: Sentinel)
Bot1.Pass
Bot2.Pass
[T3 Bot1.Main2]
[Expect]
Bot1.Graveyard.Sentinel.Power = 1
Bot2.Battlefield.Grizzly Bears.Power = 2
"#, "sentinel-source-leaves").unwrap();
    let result = run_on(&script, table).map_err(|f| f.report).unwrap();
    let card = table.find("Sentinel").unwrap();
    assert!(result.game.log.iter().any(|r| matches!(r.event, Event::Resolved { ability, .. } if ability.card == card && ability.index == 0)), "Sentinel's ability must resolve after its source leaves");
}
