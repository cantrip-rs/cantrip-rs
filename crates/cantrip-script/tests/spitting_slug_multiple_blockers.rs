use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;

// Brave the Sands' relevant permission, supplied as a test fixture without adding a card to the card set.
fn one_more(q: &dyn Query, source: ObjectId, blocker: ObjectId) -> u32 {
    u32::from(q.controller(source) == q.controller(blocker))
}
static PERMISSION: CardDef = CardDef {
    name: "Block Permission Fixture", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Enchantment]),
    additional_blocks: &[one_more], ..CardDef::BLANK
};

#[test]
fn slug_blocks_two_attackers_with_one_trigger_and_affects_both() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new(cards().cards.iter().copied().chain([&PERMISSION])));
    let script = GameScript::parse(r#"# A supporting test fixture supplies Brave the Sands' additional-block permission.
[Bot1 Deck]
Spitting Slug
Black Lotus
Forest
Block Permission Fixture
Island x20
[Bot2 Deck]
Grizzly Bears
Grizzly Bears
Black Lotus
Forest
Island x20
[Actions]
Bot1.Hand.Forest.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot1.Hand.Block Permission Fixture.Cast
Bot1.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Hand.Spitting Slug.Cast
Bot1.Pass
[T2 Bot2.Main1]
Bot2.Hand.Forest.Play
Bot2.Hand.Black Lotus.Cast
Bot2.Pass
Bot2.Battlefield.Black Lotus.Activate(Color: Green)
Bot2.Hand.Grizzly Bears.Cast
Bot2.Pass
Bot2.Battlefield.Forest.Activate
Bot2.Hand.Grizzly Bears.Cast
Bot2.Pass
[T4 Bot2.Attackers]
Bot2.Attack(Grizzly Bears#1 > Bot1; Grizzly Bears#2 > Bot1)
Bot1.Block(Spitting Slug > Grizzly Bears#1; Spitting Slug > Grizzly Bears#2)
? Stack.Count = 1
Bot1.Decline
? Bot2.Battlefield.Grizzly Bears#1.Keywords = FirstStrike
? Bot2.Battlefield.Grizzly Bears#2.Keywords = FirstStrike
[T4 Bot2.Main2]
[Expect]
Bot1.Life = 20
"#, "slug-multiblock").unwrap();
    let game = run_on(&script, cards).unwrap_or_else(|failure| panic!("{}", failure.report)).game;
    let blocks: Vec<_> = game.history().all().iter().filter_map(|r| match r.event {
        Event::Blocks { blocker, attacker, declared } if game.name(blocker) == "Spitting Slug" => Some((attacker, declared)),
        _ => None,
    }).collect();
    assert_eq!(blocks.len(), 2, "both attacker relationships were recorded");
    assert_ne!(blocks[0].0, blocks[1].0);
    assert_eq!(blocks.iter().filter(|(_, declared)| *declared).count(), 1);
}
