//! A negative target assertion can name a real stack ability even if it is not offered.
use cantrip_engine::cards;
use cantrip_script::{GameScript, ScriptFailure, run_on};

fn play(actions: &str) -> Result<(), ScriptFailure> { play_with(actions, false) }

fn play_with(actions: &str, blue: bool) -> Result<(), ScriptFailure> {
    let text = format!("[Bot1 Deck]\nPlains\nBlack Lotus\nMox Pearl\nThraben Inspector\nAang's Iceberg\nTishana's Tidebinder\nPlains x20\n[Bot2 Deck]\nIsland x20\n[Actions]\nBot1.Hand.Plains.Play\nBot1.Hand.Black Lotus.Cast\nBot1.Hand.Mox Pearl.Cast\nBot1.Battlefield.Plains.Activate\nBot1.Hand.Thraben Inspector.Cast\nBot1.Pass\nBot2.Pass\n? Stack = Thraben Inspector\nBot1.Battlefield.Black Lotus.Activate(Color: White)\n{actions}\n[Expect]\n");
    let text = if blue { text.replace("Color: White", "Color: Blue") } else { text };

    run_on(&GameScript::parse(&text, "refused-stack-ability").unwrap(), cards()).map(|_| ())
}
const ICEBERG: &str = "Bot1.Hand.Aang's Iceberg.Cast\nBot1.Pass\nBot2.Pass\n";
#[test]
fn existing_ability_that_is_not_a_candidate_can_be_refused() {
    play(&format!("{ICEBERG}! Bot1.Target(Stack.Thraben Inspector)\nBot1.Target(Mox Pearl)\n")).unwrap();
}
#[test]
fn absent_ability_is_not_a_meaningful_refusal() {
    let e=play(&format!("{ICEBERG}! Bot1.Target(Stack.Llanowar Elves)\nBot1.Target(Mox Pearl)\n")).expect_err("nothing exists");
    assert!(e.message.contains("proves nothing"), "{}",e.report);
}

#[test]
fn legal_stack_ability_cannot_be_refused() {
    let e=play_with("Bot1.Hand.Tishana's Tidebinder.Cast\nBot1.Pass\nBot2.Pass\n! Bot1.Target(Stack.Thraben Inspector)\n",true).expect_err("ability is a legal candidate");
    assert!(e.message.contains("must not be a legal target"), "{}",e.report);
}
#[test]
fn positive_stack_ability_selection_still_resolves() {
    play_with("Bot1.Hand.Tishana's Tidebinder.Cast\nBot1.Pass\nBot2.Pass\nBot1.Target(Stack.Thraben Inspector)\n",true).unwrap();
}
#[test]
fn same_named_spell_does_not_hide_an_illegal_ability() {
    let script = GameScript::parse(include_str!("fixtures/refused_stack_abilities/duplicate-iceberg-illegal.cantrip"), "same-name-illegal").unwrap();
    run_on(&script, cards()).unwrap();
}
#[test]
fn same_named_spell_does_not_hide_a_legal_ability() {
    let script = GameScript::parse(include_str!("fixtures/refused_stack_abilities/duplicate-iceberg.cantrip"), "same-name-legal").unwrap();
    let e = run_on(&script, cards()).err().expect("the ability is a legal candidate");
    assert!(e.message.contains("must not be a legal target"), "{}", e.report);
}
#[test]
fn positive_same_named_stack_ability_is_still_selected() {
    let text = include_str!("fixtures/refused_stack_abilities/duplicate-iceberg.cantrip")
        .replace("! Bot1.Target(Stack.Aang's Iceberg)", "Bot1.Target(Stack.Aang's Iceberg)");
    let script = GameScript::parse(&text, "same-name-positive").unwrap();
    run_on(&script, cards()).unwrap();
}
#[test]
fn legal_spell_does_not_stand_in_for_an_illegal_same_named_ability() {
    let original = include_str!("fixtures/refused_stack_abilities/spell-vs-ability.cantrip").to_owned();
    let negative = GameScript::parse(&original, "spell-versus-ability-negative").unwrap();
    run_on(&negative, cards()).unwrap();
    let positive = original.replace("! Bot1.Target(Stack.Aang's Iceberg)", "Bot1.Target(Stack.Aang's Iceberg)")
        .replace("Bot1.Target(Bot1.Stack.Aang's Iceberg)\n", "");
    let positive = GameScript::parse(&positive, "spell-versus-ability-positive").unwrap();
    let e = run_on(&positive, cards()).err().expect("the named ability is not offered");
    assert!(e.message.contains("did not become possible"), "{}", e.report);
}
#[test]
fn numbered_abilities_keep_their_stack_ordinals_when_only_one_is_offered() {
    let original = include_str!("fixtures/refused_stack_abilities/ability-ordinal.cantrip");
    let correct = GameScript::parse(original, "ability-ordinal").unwrap();
    run_on(&correct, cards()).unwrap();
    let refused_legal = original.replace("Activate(Stack.Cloistered Telepath#1)", "Activate(Stack.Cloistered Telepath#2)");
    let refused_legal = GameScript::parse(&refused_legal, "ability-ordinal-legal-refused").unwrap();
    let e = run_on(&refused_legal, cards()).err().expect("the second ability is offered");
    assert!(e.message.contains("must not be offered"), "{}", e.report);
    let positive_illegal = original.replace("! Bot1.Battlefield.Echo, Perceptive Prodigy.Activate(Stack.Cloistered Telepath#1)", "Bot1.Battlefield.Echo, Perceptive Prodigy.Activate(Stack.Cloistered Telepath#1)");
    let positive_illegal = GameScript::parse(&positive_illegal, "ability-ordinal-illegal-positive").unwrap();
    let e = run_on(&positive_illegal, cards()).err().expect("the first ability is not offered");
    assert!(e.message.contains("did not become possible"), "{}", e.report);
}
#[test]
fn missing_ability_ordinal_does_not_fall_back_to_a_same_named_spell() {
    let original = include_str!("fixtures/refused_stack_abilities/spell-vs-ability.cantrip").to_owned();
    let negative = original.replace("Stack.Aang's Iceberg)", "Stack.Aang's Iceberg#2)");
    let negative = GameScript::parse(&negative, "missing-ability-ordinal").unwrap();
    let e = run_on(&negative, cards()).err().expect("there is no second ability");
    assert!(e.message.contains("proves nothing"), "{}", e.report);
    let positive = original.replace("! Bot1.Target(Stack.Aang's Iceberg)", "Bot1.Target(Stack.Aang's Iceberg#2)")
        .replace("Bot1.Target(Bot1.Stack.Aang's Iceberg)\n", "");
    let positive = GameScript::parse(&positive, "missing-ability-ordinal-positive").unwrap();
    let e = run_on(&positive, cards()).err().expect("there is no second ability");
    assert!(e.message.contains("did not become possible"), "{}", e.report);
}
