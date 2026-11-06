//! A scenario can name a creature blocking more than one attacker (CR 509.1a).
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;

static FILLER: CardDef = CardDef { name: "Block Script Filler", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static ATTACKER: CardDef = CardDef { name: "Block Script Attacker", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(4), ..CardDef::BLANK };
static BLOCKER: CardDef = CardDef { name: "Block Script Guard", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(4), abilities: &[block_allowance(1, "This creature can block an additional creature each combat.")], ..CardDef::BLANK };

#[test]
fn one_blocker_can_be_named_again_for_another_attacker() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new([&FILLER, &ATTACKER, &BLOCKER]));
    let text = "[Bot1 Deck]\nBlock Script Attacker x2\nBlock Script Filler x18\n[Bot2 Deck]\nBlock Script Guard\nBlock Script Filler x19\n[Actions]\nBot1.Hand.Block Script Attacker.Cast\nBot1.Pass\nBot1.Hand.Block Script Attacker.Cast\nBot1.Pass\n[T2 Bot2.Main1]\nBot2.Hand.Block Script Guard.Cast\n[T3 Bot1.Attackers]\nBot1.Attack(Block Script Attacker#1 > Bot2; Block Script Attacker#2 > Bot2)\nBot2.Block(Block Script Guard > Block Script Attacker#1; Block Script Guard > Block Script Attacker#2)\n[T3 Bot1.Main2]\n[Expect]\nBot2.Life = 20\n";
    let script = GameScript::parse(text, "multi-block-script").unwrap();
    run_on(&script, cards).unwrap();
    // With no band, the defending player chooses how the blocker assigns its damage to the attackers.
    let split = text.replace("[T3 Bot1.Main2]", "[T3 Bot1.Blockers]\nBot1.Pass\nBot2.AssignDamage(Block Script Guard > Block Script Attacker#2: 1)\n[T3 Bot1.Main2]")
        .replace("Bot2.Life = 20", "Bot2.Life = 20\nBot1.Battlefield.Block Script Attacker#1.Damage = 0\nBot1.Battlefield.Block Script Attacker#2.Damage = 1");
    let split = GameScript::parse(&split, "multi-block-damage").unwrap();
    run_on(&split, cards).unwrap();
}
