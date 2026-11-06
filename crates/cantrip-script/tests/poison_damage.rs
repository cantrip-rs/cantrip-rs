//! The shared printed damage trigger covers both damage kinds and records the
//! damaged player, independently of damage amount and ability controller.
use std::sync::OnceLock;
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::{poison, targets};
use cantrip_script::{GameScript, run_on};

fn hit<const N: u16>(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.damage(item.object, Target::Player(player), u32::from(N));
    }
    Ok(())
}
fn other_hits(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let other = g.permanents_of(item.controller).into_iter()
        .find(|&id| g.card(g.chars(id).card).name == "Poison Bystander").unwrap();
    if let Some(player) = targets::player(g, item, 0) {
        g.damage(other, Target::Player(player), 4);
    }
    Ok(())
}
const TEXT: &str = "Whenever this creature deals damage to a player, that player gets a poison counter.";
static ONE: CardDef = CardDef {
    name: "Poison One", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(5),
    abilities: &[ability(Cost::NONE, hit::<4>, "Hit a player.").targets(&[targets::PLAYER]), poison::on_damage_to_player::<1>(TEXT)],
    ..CardDef::BLANK
};
static TWO: CardDef = CardDef { name: "Poison Two",
    abilities: &[ability(Cost::NONE, hit::<4>, "Hit a player.").targets(&[targets::PLAYER]), poison::on_damage_to_player::<2>("Whenever this creature deals damage to a player, that player gets two poison counters.")], ..ONE };
static ZERO: CardDef = CardDef { name: "Poison Zero Damage",
    abilities: &[ability(Cost::NONE, hit::<0>, "Hit a player.").targets(&[targets::PLAYER]), poison::on_damage_to_player::<1>(TEXT)], ..ONE };
static OTHER: CardDef = CardDef { name: "Poison Other Source",
    abilities: &[ability(Cost::NONE, other_hits, "Another creature hits a player.").targets(&[targets::PLAYER]), poison::on_damage_to_player::<1>(TEXT)], ..ONE };
static BYSTANDER: CardDef = CardDef { name: "Poison Bystander", abilities: &[], ..ONE };
static FILLER: CardDef = CardDef { name: "Poison Filler", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn play(name: &str, extra: &str, actions: &str, life: u8, poison: u8) {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new([&ONE, &TWO, &ZERO, &OTHER, &BYSTANDER, &FILLER]));
    let text = format!("[Bot1 Deck]\n{name}\n{extra}\nPoison Filler x20\n[Bot2 Deck]\nPoison Filler x20\n[Actions]\nBot1.Hand.{name}.Cast\nBot1.Pass\nBot2.Pass\n{actions}\n[Expect]\nBot2.Life = {life}\nBot2.Poison = {poison}\nBot1.Poison = 0\n");
    let script = GameScript::parse(&text, "shared-poison-damage").unwrap();
    if let Err(failure) = run_on(&script, table) {
        panic!("{}\n{}", failure.message, failure.report);
    }
}

#[test]
fn noncombat_damage_gives_one_counter_to_the_damaged_player() {
    play("Poison One", "", "Bot1.Battlefield.Poison One.Activate(Bot2)\nBot1.Pass\nBot2.Pass\n[T1 Bot1.Main2]", 16, 1);
}
#[test]
fn the_counter_count_is_the_printed_constant_not_the_damage_amount() {
    play("Poison Two", "", "Bot1.Battlefield.Poison Two.Activate(Bot2)\nBot1.Pass\nBot2.Pass\n[T1 Bot1.Main2]", 16, 2);
}
#[test]
fn zero_damage_does_not_trigger_poison() {
    play("Poison Zero Damage", "", "Bot1.Battlefield.Poison Zero Damage.Activate(Bot2)\nBot1.Pass\nBot2.Pass\n[T1 Bot1.Main2]", 20, 0);
}
#[test]
fn another_source_dealing_damage_does_not_trigger_this_ability() {
    play("Poison Other Source", "Poison Bystander", "Bot1.Hand.Poison Bystander.Cast\nBot1.Pass\nBot2.Pass\nBot1.Battlefield.Poison Other Source.Activate(Bot2)\nBot1.Pass\nBot2.Pass\n[T1 Bot1.Main2]", 16, 0);
}
#[test]
fn combat_damage_uses_the_same_trigger() {
    play("Poison One", "", "[T3 Bot1.DeclareAttackers]\nBot1.Attack(Poison One)\n[T3 Bot1.Main2]", 19, 1);
}
