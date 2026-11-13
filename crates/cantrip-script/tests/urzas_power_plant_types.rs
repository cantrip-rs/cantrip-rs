//! Test-only lands with the relevant printed types exercise the real Power Plant.
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;

static MINE: CardDef = CardDef { name: "Probe Mine", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Urzas, Subtype::Mine]), ..CardDef::BLANK };
static BARE_MINE: CardDef = CardDef { name: "Bare Mine", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Mine]), ..CardDef::BLANK };
static BARE_TOWER: CardDef = CardDef { name: "Bare Tower", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Tower]), ..CardDef::BLANK };
static URZA_MINE: CardDef = CardDef { name: "Urza's Mine", ..MINE };
static URZA_TOWER: CardDef = CardDef { name: "Urza's Tower", ..TOWER };
static TOWER: CardDef = CardDef { name: "Probe Tower", oracle: true, types: TypeSet::of(&[CardType::Land]), subtypes: SubtypeSet::of(&[Subtype::Urzas, Subtype::Tower]), ..CardDef::BLANK };

fn play(text: &str) {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new([&MINE, &TOWER, &BARE_MINE, &BARE_TOWER, &URZA_MINE, &URZA_TOWER].into_iter().chain(cantrip_cards::all().into_iter().filter(|card| ![&MINE, &TOWER, &BARE_MINE, &BARE_TOWER, &URZA_MINE, &URZA_TOWER].iter().any(|probe| probe.name == card.name)))));
    let script = GameScript::parse(text, "power-plant-types").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
}

#[test]
fn differently_named_lands_of_both_types_add_two() {
    play("[Bot1 Deck]\nUrza's Power Plant\nProbe Mine\nProbe Tower\nIsland x20\n[Bot2 Deck]\nIsland x20\n[Actions]\nBot1.Hand.Urza's Power Plant.Play\n[T3 Bot1.Main1]\nBot1.Hand.Probe Mine.Play\n[T5 Bot1.Main1]\nBot1.Hand.Probe Tower.Play\nBot1.Battlefield.Urza's Power Plant.Activate\n? Bot1.Mana = {C}{C}\n[Expect]\nBot1.Battlefield.Urza's Power Plant.Tapped = true\n");
}

#[test]
fn an_opponents_tower_does_not_count() {
    play("[Bot1 Deck]\nUrza's Power Plant\nProbe Mine\nIsland x20\n[Bot2 Deck]\nProbe Tower\nIsland x20\n[Actions]\nBot1.Hand.Urza's Power Plant.Play\n[T2 Bot2.Main1]\nBot2.Hand.Probe Tower.Play\n[T3 Bot1.Main1]\nBot1.Hand.Probe Mine.Play\nBot1.Battlefield.Urza's Power Plant.Activate\n? Bot1.Mana = {C}\n[Expect]\nBot1.Battlefield.Urza's Power Plant.Tapped = true\n");
}

#[test]
fn an_opponents_mine_does_not_count() {
    play("[Bot1 Deck]\nUrza's Power Plant\nProbe Tower\nIsland x20\n[Bot2 Deck]\nProbe Mine\nIsland x20\n[Actions]\nBot1.Hand.Urza's Power Plant.Play\n[T2 Bot2.Main1]\nBot2.Hand.Probe Mine.Play\n[T3 Bot1.Main1]\nBot1.Hand.Probe Tower.Play\nBot1.Battlefield.Urza's Power Plant.Activate\n? Bot1.Mana = {C}\n[Expect]\nBot1.Battlefield.Urza's Power Plant.Tapped = true\n");
}

#[test]
fn original_scenario_with_unimplemented_companion_lands() {
    play(include_str!("fixtures/urzas-power-plant-urza-lands-add-two.cantrip"));
}

#[test]
fn mine_and_tower_without_urzas_type_do_not_count() {
    play("[Bot1 Deck]\nUrza's Power Plant\nBare Mine\nBare Tower\nIsland x20\n[Bot2 Deck]\nIsland x20\n[Actions]\nBot1.Hand.Urza's Power Plant.Play\n[T3 Bot1.Main1]\nBot1.Hand.Bare Mine.Play\n[T5 Bot1.Main1]\nBot1.Hand.Bare Tower.Play\nBot1.Battlefield.Urza's Power Plant.Activate\n? Bot1.Mana = {C}\n[Expect]\nBot1.Battlefield.Urza's Power Plant.Tapped = true\n");
}
