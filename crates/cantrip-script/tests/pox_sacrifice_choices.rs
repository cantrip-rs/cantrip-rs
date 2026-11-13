//! A protected creature land and an eligible creature and land exercise each sacrifice stage.
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;

// Hithlain Rope can acquire both creature and land types, for example through March of the Machines and Ashaya.
static GUARDED: CardDef = CardDef {
    name: "Pox protected creature land", oracle: true,
    types: TypeSet::of(&[CardType::Creature, CardType::Land]),
    power: Some(1), toughness: Some(1),
    keywords: Keywords::of(&[Keyword::CantBeSacrificed]),
    ..CardDef::BLANK
};

#[test]
fn protected_permanent_does_not_create_a_sacrifice_choice() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| { let mut cards = cantrip_cards::all(); cards.push(&GUARDED); CardTable::new(cards) });
    let script = GameScript::parse(r#"
[Bot1 Deck]
Pox protected creature land
Memnite
Swamp
Black Lotus
Pox
Island
Island
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Pox protected creature land.Play
Bot1.Hand.Memnite.Cast
Bot1.Hand.Black Lotus.Cast
[T3 Bot1.Main1]
Bot1.Hand.Swamp.Play
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Pox.Cast
Bot1.Pass
Bot2.Pass
Bot1.Choose(Island)
Bot2.Choose(Island; Island; Island)
! Bot1.Choose
[Expect]
Bot1.Battlefield.Pox protected creature land.Types = Land; Creature
Bot1.Battlefield.Memnite = (absent)
Bot1.Battlefield.Swamp = (absent)
"#, "Pox protected choices").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}
