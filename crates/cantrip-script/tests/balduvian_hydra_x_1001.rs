//! Forced mana makes the ordinary X-announcement bound observable without a loop or seed search.
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn enough_mana(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::C, 1001);
    g.end_sentence();
    g.add_mana(item.controller, Mana::R, 2);
    Ok(())
}
static MANA: CardDef = CardDef {
    name: "Hydra mana probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], enough_mana), ..CardDef::BLANK
};
#[test]
fn ordinary_x_can_be_1001_when_mana_is_available() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MANA])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Hydra mana probe
Balduvian Hydra
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Hydra mana probe.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Balduvian Hydra.Cast(X: 1001)
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Balduvian Hydra.Power = 1001
"#, "balduvian-hydra-x-1001").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}

#[test]
fn same_mana_setup_can_cast_x_1000() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MANA])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Hydra mana probe
Balduvian Hydra
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Hydra mana probe.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Balduvian Hydra.Cast(X: 1000)
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Balduvian Hydra.Power = 1000
"#, "balduvian-hydra-x-1000").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}
