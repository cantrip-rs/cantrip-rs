//! A returned static effect makes the damaged Vampire lethal. SBAs wait until the resolving ability finishes (CR 704.3).
use std::sync::OnceLock;
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn nonwhite_creature(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature) && !c.colors.has(Color::White)
}
fn minus_one(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.power = c.power.map(|n| n - 1);
    c.toughness = c.toughness.map(|n| n - 1);
}
// Crovax's relevant static ability, supplied as a test-only card: it is white and does not shrink itself.
static HERO: CardDef = CardDef {
    name: "Probe White Hero", oracle: true, has_cost: true, colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]), power: Some(4), toughness: Some(4),
    statics: &[static_ability(Layer::PtModify, nonwhite_creature, minus_one, "Nonwhite creatures get -1/-1.")],
    ..CardDef::BLANK
};
fn mark_and_kill(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let battlefield = g.state().battlefield();
    let vampire = battlefield.iter().copied().find(|&id| g.chars(id).name == "Krovikan Vampire");
    let hero = battlefield.iter().copied().find(|&id| g.chars(id).name == "Probe White Hero");
    if let (Some(vampire), Some(hero)) = (vampire, hero) {
        g.damage(vampire, Target::Object(hero), 1);
        g.end_sentence();
        g.damage(item.object, Target::Object(hero), 3);
    }
    Ok(())
}
static STRIKE: CardDef = CardDef { name: "Probe Mark and Kill", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], mark_and_kill), ..CardDef::BLANK };
fn hero_is_yours(q: &dyn Query, asker: &Asker) -> bool {
    q.state().battlefield().iter().any(|&id| q.chars(id).name == "Probe White Hero" && q.controller(id) == asker.controller)
}
fn mark_watcher(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Charge, 1);
    Ok(())
}
static WATCHER: CardDef = CardDef {
    name: "Probe State Watcher", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[state_triggered(hero_is_yours, mark_watcher, "When you control Probe White Hero, put a charge counter on this.")],
    ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&HERO, &STRIKE, &WATCHER])))
}
#[test]
fn returned_static_effect_kills_vampire_only_after_delayed_trigger_exists() {
    let text = r#"
[Bot1 Deck]
Krovikan Vampire
Black Lotus
Mox Jet
Mox Pearl
Mox Ruby
Probe Mark and Kill
Shock
Probe State Watcher
Island x20
[Bot2 Deck]
Probe White Hero
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Krovikan Vampire.Cast
Bot1.Pass
[T2 Bot2.Main1]
Bot2.Hand.Probe White Hero.Cast
Bot2.Pass
[T3 Bot1.Main1]
Bot1.Hand.Probe State Watcher.Cast
? Bot1.Battlefield.Krovikan Vampire.Toughness = 2
Bot1.Hand.Probe Mark and Kill.Cast
Bot1.Pass
Bot2.Pass
? Bot2.Graveyard.Probe White Hero.Toughness = 4
? Bot1.Battlefield.Krovikan Vampire.Toughness = 3
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Shock.Cast(Bot1.Battlefield.Krovikan Vampire)
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield.Krovikan Vampire.Damage = 2
[T4 Bot2.Main1]
[Expect]
Bot1.Battlefield.Krovikan Vampire = (absent)
Bot1.Battlefield.Probe White Hero = (absent)
Bot1.Graveyard.Krovikan Vampire.Power = 3
Bot2.Graveyard.Probe White Hero.Toughness = 4
Bot1.Battlefield.Probe State Watcher.Counters.Charge = 1
"#;
    let script = GameScript::parse(text, "Krovikan Vampire sentence boundary").unwrap();
    if let Err(failure) = run_on(&script, table()) { panic!("{}", failure.report); }
}
