//! A creature put onto the battlefield attacking was never declared as an attacker (CR 508.4): it has no `Event::Attacks`,
//! so `History::attacked_with` - "you attacked with a Rogue" (Robber of the Rich), "attacked with N creatures" (Windbrisk
//! Heights) - does not give it, though it is an attacking creature.

use cantrip_engine::*;
use cantrip_script::GameScript;

/// Ornithopter attacks alone on turn 3, unblocked, and Moon-Circuit Hacker is put onto the battlefield tapped and
/// attacking by ninjutsu in its place (CR 702.49a); Bot1 declines the Hacker's draw.
const NINJUTSU: &str = "
[Bot1 Deck]
Mox Sapphire
Ornithopter
Moon-Circuit Hacker
Island x4
Island x10

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Hand.Ornithopter.Cast
Bot1.Pass
[T3 Bot1.Main1]
Bot1.Attack(Ornithopter)
[T3 Bot1.Blockers]
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Moon-Circuit Hacker.Activate(Ability: Ninjutsu)
Bot1.Choose(Pay mana)
Bot1.Choose(Ornithopter)
Bot1.Pass
Bot1.Pass
Bot1.Decline
[T3 Bot1.Main2]

[Expect]
Turn = 3
Step = Main2
Bot2.Life = 18
Bot1.Hand = Island; Island; Island; Island; Island; Ornithopter
Bot1.Battlefield.Moon-Circuit Hacker.Tapped = true
";

#[test]
fn a_creature_put_onto_the_battlefield_attacking_was_not_attacked_with() {
    let script = GameScript::parse(NINJUTSU, "ninjutsu").expect("parses");
    let game = cantrip_script::run(&script).map_err(|f| f.report).expect("plays").game;
    let bot1 = PlayerId(0);
    let hacker = game.state.battlefield().iter().copied().find(|&id| game.cards.get(game.state.obj(id).card).name == "Moon-Circuit Hacker").expect("the Hacker");
    let declared: Vec<ObjectId> = game.history().attacked_with(bot1).map(|a| a.attacker).collect();
    assert_eq!(declared.len(), 1, "the Ornithopter alone was declared: {declared:?}");
    assert_eq!(game.cards.get(game.state.obj(declared[0]).card).name, "Ornithopter");
    assert!(!declared.contains(&hacker), "the Hacker was put onto the battlefield attacking, not declared (CR 508.4)");
    let attacks = game.history().this_turn().iter().filter(|r| matches!(r.event, Event::Attacks { attacker, .. } if attacker == hacker)).count();
    assert_eq!(attacks, 0, "no Event::Attacks for the Hacker");
}
