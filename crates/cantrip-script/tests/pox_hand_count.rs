//! A token temporarily in hand during resolution is not a card (CR 111.6, 704.5d).
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;

fn replaces(_: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::LifeChange(change) if change.player == asker.controller && change.amount < 0)
}
fn replace(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::LifeChange(change) = p { change.amount = 0; }
}
fn token_to_hand(g: &mut dyn Rules, asker: &Asker, _: &Proposal, _: &Proposal) {
    let id = g.card_id(&TOKEN);
    let tokens = g.create_tokens(id, asker.controller, 1, false).unwrap();
    g.return_to_hand(&tokens);
}
static TOKEN: CardDef = CardDef {
    name: "Pox probe token", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK
};
// Test-only replacement creates a noncard in hand during the first instruction, before SBAs can remove it.
static FIXTURE: CardDef = CardDef {
    name: "Pox hand probe", oracle: true, types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, replaces, replace, "Return a token instead").then(token_to_hand)],
    ..CardDef::BLANK
};
#[test]
fn temporary_token_is_not_counted_as_a_card() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| { let mut cards = cantrip_cards::all(); cards.extend([&FIXTURE, &TOKEN]); CardTable::new(cards) });
    let text = r#"
[Bot1 Deck]
Pox hand probe
Black Lotus
Pox
Mox Ruby
Mox Pearl
Island x20
[Bot2 Deck]
Island x20
[Bot3 Deck]
Island x20
[Actions]
Bot1.Hand.Pox hand probe.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Pox.Cast
Bot1.Pass
Bot2.Pass
Bot3.Pass
! Bot1.Choose(Pox probe token)
Bot1.Choose(Island)
[Expect]
Bot1.Hand.Count = 2
"#;
    let script = GameScript::parse(text, "Pox counts cards").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
    let prefix = text.split("! Bot1.Choose(Pox probe token)").next().unwrap();
    let handover = GameScript::parse(&format!("{prefix}[Handover Bot1.Choose]\n"), "Pox discard prompt").unwrap();
    let result = run_on(&handover, table).map_err(|f| f.report).unwrap();
    let RequestKind::ChooseCards { purpose, rule, .. } = &result.game.pending().unwrap().kind else { panic!("expected discard choice") };
    assert!(matches!(purpose, Purpose::Discard));
    assert!(matches!(rule.count, Count::Exactly(1)));
}
