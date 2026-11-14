//! A creature with no power value is treated as power zero (CR 208.5).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

static NO_POWER: CardDef = CardDef {
    name: "Sentinel Test No Power", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Creature]), power: None, toughness: Some(1),
    ..CardDef::BLANK
};

#[test]
fn missing_power_is_zero_for_the_real_sentinel_ability() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&NO_POWER])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus
Mox Pearl
Sentinel
Island x20
[Bot2 Deck]
Sentinel Test No Power
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Sentinel.Cast
[T2 Bot2.Main1]
Bot2.Hand.Sentinel Test No Power.Cast
[T3 Bot1.DeclareAttackers]
Bot1.Attack(Sentinel > Bot2)
Bot2.Block(Sentinel Test No Power > Sentinel)
Bot1.Battlefield.Sentinel.Activate(Bot2.Battlefield.Sentinel Test No Power)
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield.Sentinel.Toughness = 1
[T3 Bot1.Main2]
[Expect]
Bot1.Battlefield.Sentinel.Toughness = 1
"#, "sentinel-missing-power").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|failure| failure.report).unwrap_or_default());
}
