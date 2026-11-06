//! A draw replacement can change state without an event (CR 614.6). State triggers must see the completed draw
//! instruction before the subsequent discard changes the state again (CR 603.8, 608.2c). These test-only cards isolate
//! the shared draw-then-discard recipe used by Currency Converter, Sphinx of False Conclusions and Jalum Tome.
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;

fn draw_for_you(_: &dyn Query, a: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Draw(d) if d.player == a.controller)
}
fn replace_draw(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Draw(d) = p {
        d.replaced = true;
    }
}
fn mark(g: &mut dyn Rules, a: &Asker, _: &Proposal, _: &Proposal) {
    g.add_effect(
        a.source,
        a.source,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),
        Duration::EndOfTurn,
    );
}
fn marked_with_one_card(q: &dyn Query, a: &Asker) -> bool {
    q.chars(a.source).keywords.has(Keyword::Flying)
        && q.state().zone(Zone::hand(a.controller)).len() == 1
}
fn gain_three(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}
static MARKER: CardDef = CardDef {
    name: "Probe Draw Marker",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(
        ReplacementKind::Draw,
        draw_for_you,
        replace_draw,
        "If you would draw a card, this enchantment gains flying until end of turn instead.",
    )
    .then(mark)],
    abilities: &[state_triggered(
        marked_with_one_card,
        gain_three,
        "When this enchantment has flying and you have exactly one card in hand, you gain 3 life.",
    )],
    ..CardDef::BLANK
};
static LOOTER: CardDef = CardDef {
    name: "Probe Looter",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(
        TAP,
        cantrip_mechanics::effects::loot,
        "{T}: Draw a card, then discard a card.",
    )],
    ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MARKER, &LOOTER])))
}
#[test]
fn state_true_after_replaced_draw_is_seen_before_discard() {
    let script = GameScript::parse(
        r#"
[Bot1 Deck]
Probe Draw Marker
Probe Looter
Mox Pearl
Mox Sapphire
Mox Jet
Mox Ruby
Island
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Probe Draw Marker.Cast
Bot1.Hand.Probe Looter.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Hand.Mox Ruby.Cast
? Bot1.Hand.Count = 1
Bot1.Battlefield.Probe Looter.Activate
Bot1.Choose(Bot1.Hand.Island)
[Expect]
Bot1.Hand.Count = 0
Bot1.Life = 23
"#,
        "loot-replaced-draw-state-boundary",
    )
    .unwrap();
    if let Err(failure) = run_on(&script, table()) {
        panic!("{}", failure.report);
    }
}

#[test]
fn normal_draw_then_discard_keeps_the_other_cards() {
    let script = GameScript::parse(
        r#"
[Bot1 Deck]
Probe Looter
Island x6
Mountain
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Probe Looter.Cast
? Bot1.Hand.Count = 6
Bot1.Battlefield.Probe Looter.Activate
Bot1.Choose(Bot1.Hand.Mountain)
[Expect]
Bot1.Hand.Count = 6
Bot1.Hand = Island; Island; Island; Island; Island; Island
Bot1.Graveyard = Mountain
Bot1.Life = 20
"#,
        "loot-normal-draw-discard-control",
    )
    .unwrap();
    if let Err(failure) = run_on(&script, table()) {
        panic!("{}", failure.report);
    }
}
