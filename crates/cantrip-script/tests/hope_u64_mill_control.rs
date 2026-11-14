//! A real Hope Estheim trigger consumes a life-gained history exceeding u32.
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn gain_then_hope(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_life_total(item.controller, -1);
    g.end_sentence();
    g.set_life_total(PlayerId(1), i32::MAX);
    g.end_sentence();
    for _ in 0..3 {
        cantrip_mechanics::exchange::life_totals(g, item.controller, PlayerId(1));
        g.end_sentence();
    }
    assert_eq!(g.history().life_gained(item.controller), 1u64 << 32);
    let hope = cantrip_cards::all().into_iter().find(|c| c.name == "Hope Estheim").unwrap();
    let AbilityDef::Triggered(trigger) = hope.abilities[0] else { panic!("Hope's triggered ability") };
    (trigger.effect)(g, item)
}
static PROBE: CardDef = CardDef {
    name: "Hope history probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], gain_then_hope), ..CardDef::BLANK
};
#[test]
fn hope_mills_a_finite_library_when_gained_life_is_2_to_the_32() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&PROBE])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Hope history probe
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Hope history probe.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Library.Count = 0
Bot2.Graveyard.Count = 13
"#, "hope-u64-history").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}

fn wide_mill_applies(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Mill(m) if m.count == 1u64 << 32)
}
fn reduce_wide_mill(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Mill(m) = p { m.count = 1; }
}
static REPLACER: CardDef = CardDef {
    name: "Wide mill replacement probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Mill, wide_mill_applies, reduce_wide_mill, "mill one instead")],
    ..CardDef::BLANK
};
#[test]
fn replacement_sees_full_instruction_before_library_limit() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&PROBE, &REPLACER])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Hope history probe
Wide mill replacement probe
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Wide mill replacement probe.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Hope history probe.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot2.Library.Count = 12
Bot2.Graveyard.Count = 1
"#, "hope-u64-replacement").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}
