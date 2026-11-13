//! A test-only creature supplies two bands-with-other abilities and ordinary banding.
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn quality(_: &dyn Query, _: ObjectId) -> bool { true }
static SUBJECT: CardDef = CardDef {
    name: "Probe Two Bands", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Banding]),
    abilities: &[
        cantrip_mechanics::banding::with_others(quality, "Bands with other legendary creatures."),
        cantrip_mechanics::banding::with_others(quality, "Bands with other creatures named Wolves of the Hunt."),
    ], ..CardDef::BLANK
};
fn check(g: &dyn Rules, expected: usize) {
    let id = g.state().battlefield().iter().copied().find(|&id| g.card(g.obj(id).card).name == "Probe Two Bands").unwrap();
    let chars = g.chars(id);
    assert!(chars.has(Keyword::Banding), "ordinary banding remains");
    let card = g.obj(id).card;
    for index in 0..2 {
        assert_eq!(chars.abilities().any(|a| a == AbilityRef { card, index }), expected == 2, "bands-with-other ability {index}");
    }
}
fn present(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> { check(g, 2); Ok(()) }
fn absent(g: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> { check(g, 0); Ok(()) }
static PRESENT: CardDef = CardDef { name: "Probe Bands Present", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], present), ..CardDef::BLANK };
static ABSENT: CardDef = CardDef { name: "Probe Bands Absent", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], absent), ..CardDef::BLANK };

#[test]
fn removes_both_bands_with_other_but_not_banding_until_cleanup() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&SUBJECT, &PRESENT, &ABSENT]))));
    let text = r#"
[Bot1 Deck]
Shelkin Brownie
Mox Emerald
Mox Pearl
Probe Two Bands
Probe Bands Present
Probe Bands Absent
Probe Bands Present
Forest x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Mox Emerald.Cast
Bot1.Pass
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot1.Hand.Probe Two Bands.Cast
Bot1.Pass
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Shelkin Brownie.Cast
Bot1.Pass
[T3 Bot1.Main1]
Bot1.Hand.Probe Bands Present.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Shelkin Brownie.Activate(Bot1.Battlefield.Probe Two Bands)
Bot1.Pass
Bot2.Pass
Bot1.Hand.Probe Bands Absent.Cast
Bot1.Pass
Bot2.Pass
[T5 Bot1.Main1]
Bot1.Hand.Probe Bands Present.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Probe Two Bands.Keywords = Banding
"#;
    let script = GameScript::parse(text, "shelkin-brownie-bands").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
}
