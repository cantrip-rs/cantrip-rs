//! Script numbers retain their full native domain, including individual refused answers.
use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn resolve(g: &mut dyn Rules, s: &StackItem) -> Result<(), Suspend> {
    let n = cantrip_mechanics::choose::number(g, s.controller, s.object, 300, u32::MAX, "Choose a number")?;
    // Observe the exact answer without turning its value into a bounded life/counter quantity.
    g.gain_life(s.controller, if n == u32::MAX { 2 } else if n == 65535 { 1 } else { 3 });
    Ok(())
}
static SOURCE: CardDef = CardDef { name: "Number probe", oracle: true, types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], resolve), ..CardDef::BLANK };
static FILLER: CardDef = CardDef { name: "Filler", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

#[test]
fn wide_number_scripts_keep_exact_answers_and_individual_refusals() {
    let table = Box::leak(Box::new(CardTable::new([&SOURCE, &FILLER])));
    for (number, life) in [(65535u32, 21), (u32::MAX, 22)] {
        let text = format!("[Bot1 Deck]\nNumber probe\nFiller x19\n[Bot2 Deck]\nFiller x20\n[Actions]\nBot1.Hand.Number probe.Cast\n! Bot1.Number(299)\nBot1.Number({number})\n[T1 Bot1.Main2]\n[Expect]\nBot1.Life = {life}\n");
        let script = cantrip_script::GameScript::parse(&text, "wide-number-script").unwrap();
        if let Err(failure) = cantrip_script::run_on(&script, table) { panic!("{}", failure.report); }
    }
}
