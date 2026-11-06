//! "Each creature card in your hand has ninjutsu {2}{U}{B}" (Satoru Umezawa) gives a card in hand an ability of the class
//! "ninjutsu abilities" (CR 702.49a, `ActivatedDef::ninjutsu`) that its own card doesn't print: it comes from the giver's
//! `delayed` list, and "ninjutsu abilities you activate cost {1} less to activate" (Silver-Fur Master) has to reach it there.
//! Neither card is in the engine yet, so these are test-only cards built with what those two would use.

use cantrip_engine::*;
use cantrip_mechanics::{ninjutsu::ninjutsu, statics::grant_delayed_ability};
use cantrip_script::GameScript;

/// "Ninjutsu abilities you activate cost {1} less to activate."
fn one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Ability(_, ability, _) = costed else { return CostDelta::NONE };
    if player == q.controller(source) && q.activated_ability(ability).is_some_and(|d| d.ninjutsu) { CostDelta::less(1) } else { CostDelta::NONE }
}

static MASTER: CardDef = CardDef {
    name: "Granted Ninjutsu Master",
    oracle: true,
    has_cost: true,
    cost: mana("{0}"),
    types: TypeSet::of(&[CardType::Artifact]),
    text: "Ninjutsu abilities you activate cost {1} less to activate.",
    cost_changes: &[one_less],
    ..CardDef::BLANK
};

fn creature_card_in_your_hand(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let s = q.state();
    so_far.is(CardType::Creature) && s.obj(affected).zone == Zone::hand(s.obj(source).controller)
}

/// "Each creature card in your hand has ninjutsu {2}{U}{B}."
static GIVER: CardDef = CardDef {
    name: "Granted Ninjutsu Giver",
    oracle: true,
    has_cost: true,
    cost: mana("{0}"),
    types: TypeSet::of(&[CardType::Artifact]),
    text: "Each creature card in your hand has ninjutsu {2}{U}{B}.",
    statics: &[static_ability(Layer::Ability, creature_card_in_your_hand, grant_delayed_ability, "Each creature card in your hand has ninjutsu {2}{U}{B}.").everywhere()],
    delayed: &[ninjutsu(mana("{2}{U}{B}"), "Ninjutsu {2}{U}{B}")],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new(cards().cards.iter().copied().chain([&MASTER, &GIVER]))))
}

/// Bot1 has three mana, the giver and `permanents` on the battlefield and Grizzly Bears in hand, and Ornithopter attacking alone
/// on turn 5, unblocked; then `lines` are played in the declare blockers step, and `expect` judged.
fn play(permanents: &[&str], lines: &str, expect: &str) {
    let deck: String = permanents.iter().map(|p| format!("{p}\n")).collect();
    let casts: String = permanents.iter().map(|p| format!("Bot1.Hand.{p}.Cast\nBot1.Pass\n")).collect();
    let script = format!(
        "[Bot1 Deck]
{deck}Granted Ninjutsu Giver
Island
Mox Jet
Mox Sapphire
Ornithopter
Grizzly Bears
Island x13

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Island.Play
{casts}Bot1.Hand.Granted Ninjutsu Giver.Cast
Bot1.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
[T5 Bot1.Main1]
Bot1.Attack(Ornithopter)
[T5 Bot1.Blockers]
{lines}

[Expect]
Turn = 5
{expect}
"
    );
    let script = GameScript::parse(&script, "granted ninjutsu cost").expect("parses");
    cantrip_script::run_on(&script, table()).map_err(|f| f.report).expect("plays");
}

#[test]
fn a_granted_ninjutsu_ability_is_one_of_the_class_a_cost_change_names() {
    // With three mana and no reduction, the {2}{U}{B} isn't offered (CR 602.2b).
    play(&[], "! Bot1.Hand.Grizzly Bears.Activate(Ability: Ninjutsu; Mana: short)", "");
    // With it, those three mana pay it exactly, and the card is put onto the battlefield.
    play(
        &["Granted Ninjutsu Master"],
        "Bot1.Hand.Grizzly Bears.Activate(Ability: Ninjutsu)
Bot1.Battlefield.Mox Jet.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Island.Activate
Bot1.Choose(Return to hand 1 permanent)
Bot1.Choose(Ornithopter)
Bot1.Pass",
        "Bot1.Battlefield.Grizzly Bears.Tapped = true\nBot1.Mana = ",
    );
}
