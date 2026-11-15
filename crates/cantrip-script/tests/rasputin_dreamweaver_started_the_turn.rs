//! Rasputin Dreamweaver: "if Rasputin started the turn untapped" is false of a Rasputin that entered this turn (CR 400.7),
//! which only an additional upkeep after it entered can ask (Sphinx of the Second Sun, Paradox Haze; this zero-cost test
//! instant isolates the added upkeep, CR 500.8, 500.10). And it enters with exactly seven dream counters, not more brought
//! back to seven by the state-based action (CR 614.1c, 704.5r): an opponent's Vorinclex, Monstrous Raider would halve a number
//! more than seven differently.
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn extra_upkeep(g: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    g.add_phases(PhaseAnchor::Current, &[PhaseSpec::Only(Step::Upkeep)]);
    Ok(())
}
static EXTRA: CardDef = CardDef {
    name: "Probe Extra Upkeep", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Instant]), spell: spell(&[], extra_upkeep), ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&EXTRA])))
}

const CAST: &str = r#"
[Bot1 Deck]
Rasputin Dreamweaver
Black Lotus
Mox Pearl
Mox Sapphire
Mox Jet
Probe Extra Upkeep
Probe Extra Upkeep
Island x20
[Bot2 Deck]
Island x25
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot1.Hand.Mox Pearl.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Rasputin Dreamweaver.Cast
Bot1.Pass
"#;

/// Cast on turn 1, Rasputin gives two dream counters for mana: five. An upkeep added that turn finds it entered this turn, so
/// no counter (five still). On turn 3 it started the turn untapped: its own upkeep gives one (six), and an added upkeep that
/// turn another (seven) - the added upkeep alone is no reason to refuse it.
#[test]
fn an_added_upkeep_gives_no_counter_to_a_rasputin_that_entered_this_turn() {
    let text = format!("{CAST}{}", r#"Bot1.Battlefield.Rasputin Dreamweaver.Activate(Ability: Add)
Bot1.Battlefield.Rasputin Dreamweaver.Activate(Ability: Add)
? Bot1.Battlefield.Rasputin Dreamweaver.Counters.Dream = 5
Bot1.Hand.Probe Extra Upkeep.Cast
Bot1.Pass
[T1 Bot1.Main2]
? Bot1.Battlefield.Rasputin Dreamweaver.Counters.Dream = 5
[T3 Bot1.Main1]
? Bot1.Battlefield.Rasputin Dreamweaver.Counters.Dream = 6
Bot1.Hand.Probe Extra Upkeep.Cast
Bot1.Pass
[T3 Bot1.Main2]
[Expect]
Bot1.Battlefield.Rasputin Dreamweaver.Counters.Dream = 7
"#);
    let script = GameScript::parse(&text, "rasputin-added-upkeep").expect("scenario parses");
    let result = run_on(&script, table());
    assert!(result.is_ok(), "{}", result.err().map(|failure| failure.report).unwrap_or_default());
}

/// "Rasputin enters with seven dream counters on it": the counters put as it enters are seven, and the state-based action
/// removes none.
#[test]
fn rasputin_enters_with_exactly_seven_dream_counters() {
    let text = format!("{CAST}[Expect]\nBot1.Battlefield.Rasputin Dreamweaver.Counters.Dream = 7\n");
    let script = GameScript::parse(&text, "rasputin-enters-with-seven").expect("scenario parses");
    let result = run_on(&script, table());
    let game = match result {
        Ok(result) => result.game,
        Err(failure) => panic!("{}", failure.report),
    };
    let put: Vec<i32> = game.log.iter().filter_map(|r| match r.event {
        Event::CountersPut { object, kind: CounterKind::Dream, amount, .. } if game.name(object) == "Rasputin Dreamweaver" => Some(amount),
        _ => None,
    }).collect();
    assert_eq!(put, vec![7]);
    assert!(!game.log.iter().any(|r| matches!(r.event, Event::CountersRemoved { kind: CounterKind::Dream, .. })));
}
