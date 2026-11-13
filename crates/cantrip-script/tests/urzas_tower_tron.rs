//! Test-only lands exercise Tower's conditional mana ability.
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
static MINE: CardDef = CardDef { name: "Probe Mine", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Urzas, Subtype::Mine]), ..CardDef::BLANK };
static PLANT: CardDef = CardDef { name: "Probe Plant", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Urzas, Subtype::PowerPlant]), ..CardDef::BLANK };
static FALSE_MINE: CardDef = CardDef { name: "Urza's Mine", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Urzas]), ..CardDef::BLANK };
static FALSE_PLANT: CardDef = CardDef { name: "Urza's Power-Plant", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::PowerPlant]), ..CardDef::BLANK };
fn run(text: &str) {
    let table = Box::leak(Box::new(CardTable::new([&MINE, &PLANT, &FALSE_MINE, &FALSE_PLANT].into_iter().chain(cantrip_cards::all().into_iter().filter(|card| ![&MINE, &PLANT, &FALSE_MINE, &FALSE_PLANT].iter().any(|probe| probe.name == card.name))))));
    let script = GameScript::parse(text, "urzas-tower-tron").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
}
#[test]
fn matching_types_with_different_names_give_exactly_three() {
    run(r#"
[Bot1 Deck]
Urza's Tower
Probe Mine
Probe Plant
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Urza's Tower.Play
[T3 Bot1.Main1]
Bot1.Hand.Probe Mine.Play
[T5 Bot1.Main1]
Bot1.Hand.Probe Plant.Play
Bot1.Battlefield.Urza's Tower.Activate
? Bot1.Mana = {C}{C}{C}
? Bot1.Battlefield.Urza's Tower.Tapped = true
! Bot1.Battlefield.Urza's Tower.Activate
[Expect]
Bot1.Mana = {C}{C}{C}
"#);
}
#[test]
fn names_do_not_substitute_for_the_required_types() {
    for (first, second) in [("Urza's Mine", "Probe Plant"), ("Probe Mine", "Urza's Power-Plant")] {
        run(&format!(r#"
[Bot1 Deck]
Urza's Tower
{first}
{second}
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Urza's Tower.Play
[T3 Bot1.Main1]
Bot1.Hand.{first}.Play
[T5 Bot1.Main1]
Bot1.Hand.{second}.Play
Bot1.Battlefield.Urza's Tower.Activate
? Bot1.Mana = {{C}}
[Expect]
Bot1.Mana = {{C}}
"#));
    }
}
#[test]
fn each_type_must_be_controlled_by_activator() {
    for (ours, theirs) in [("Probe Mine", "Probe Plant"), ("Probe Plant", "Probe Mine")] {
        run(&format!(r#"
[Bot1 Deck]
Urza's Tower
{ours}
Island x20
[Bot2 Deck]
{theirs}
Island x20
[Actions]
Bot1.Hand.Urza's Tower.Play
[T2 Bot2.Main1]
Bot2.Hand.{theirs}.Play
[T3 Bot1.Main1]
Bot1.Hand.{ours}.Play
Bot1.Battlefield.Urza's Tower.Activate
? Bot1.Mana = {{C}}
[Expect]
Bot1.Mana = {{C}}
"#));
    }
}
