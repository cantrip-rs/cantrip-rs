//! Test-only lands exercise Urza's Mine without adding production cards.
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

static PLANT: CardDef = CardDef { name: "Probe Plant", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Urzas, Subtype::PowerPlant]), ..CardDef::BLANK };
static TOWER: CardDef = CardDef { name: "Probe Tower", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Urzas, Subtype::Tower]), ..CardDef::BLANK };
static FALSE_PLANT: CardDef = CardDef { name: "Urza's Power-Plant", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Urzas]), ..CardDef::BLANK };
static FALSE_TOWER: CardDef = CardDef { name: "Urza's Tower", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Tower]), ..CardDef::BLANK };
fn run(text: &str) {
    let table = Box::leak(Box::new(CardTable::new([&PLANT, &TOWER, &FALSE_PLANT, &FALSE_TOWER].into_iter().chain(cantrip_cards::all().into_iter().filter(|card| ![&PLANT, &TOWER, &FALSE_PLANT, &FALSE_TOWER].iter().any(|probe| probe.name == card.name))))));
    let script = GameScript::parse(text, "urzas-mine-tron").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
}
#[test]
fn matching_subtypes_produce_exactly_two_despite_different_names() {
    run(r#"
[Bot1 Deck]
Urza's Mine
Probe Plant
Probe Tower
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Urza's Mine.Play
[T3 Bot1.Main1]
Bot1.Hand.Probe Plant.Play
[T5 Bot1.Main1]
Bot1.Hand.Probe Tower.Play
Bot1.Battlefield.Urza's Mine.Activate
? Bot1.Mana = {C}{C}
? Bot1.Battlefield.Urza's Mine.Tapped = true
! Bot1.Battlefield.Urza's Mine.Activate
[Expect]
Bot1.Mana = {C}{C}
"#);
}
#[test]
fn opponent_and_names_do_not_supply_missing_subtypes() {
    run(r#"
[Bot1 Deck]
Urza's Mine
Urza's Power-Plant
Urza's Tower
Island x20
[Bot2 Deck]
Probe Plant
Probe Tower
Island x20
[Actions]
Bot1.Hand.Urza's Mine.Play
[T2 Bot2.Main1]
Bot2.Hand.Probe Plant.Play
[T3 Bot1.Main1]
Bot1.Hand.Urza's Power-Plant.Play
[T4 Bot2.Main1]
Bot2.Hand.Probe Tower.Play
[T5 Bot1.Main1]
Bot1.Hand.Urza's Tower.Play
Bot1.Battlefield.Urza's Mine.Activate
? Bot1.Mana = {C}
[Expect]
Bot1.Mana = {C}
"#);
}
#[test]
fn each_required_type_must_be_controlled_by_the_activator() {
    for (ours, theirs) in [("Probe Plant", "Probe Tower"), ("Probe Tower", "Probe Plant")] {
        run(&format!(r#"
[Bot1 Deck]
Urza's Mine
{ours}
Island x20
[Bot2 Deck]
{theirs}
Island x20
[Actions]
Bot1.Hand.Urza's Mine.Play
[T2 Bot2.Main1]
Bot2.Hand.{theirs}.Play
[T3 Bot1.Main1]
Bot1.Hand.{ours}.Play
Bot1.Battlefield.Urza's Mine.Activate
? Bot1.Mana = {{C}}
[Expect]
Bot1.Mana = {{C}}
"#));
    }
}
