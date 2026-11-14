//! A replacement redirects Tetravus's cost but the payment still counts (CR 118.11, 118.12).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn token_exiled(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ZoneChange(z) if z.from.is(ZoneKind::Battlefield) && z.to.is(ZoneKind::Exile))
}
fn return_instead(q: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ZoneChange(z) = p { z.to = Zone::hand(q.obj(z.object).owner); }
}
static REROUTER: CardDef = CardDef {
    name: "Tetravus payment redirect probe", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, token_exiled, return_instead, "return a permanent to hand instead of exiling it")],
    ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&REROUTER])))
}
#[test]
fn the_real_tetravus_counts_a_redirected_cost_payment() {
    let source = "
[Bot1 Deck]
Black Lotus
Black Lotus
Tetravus payment redirect probe
Tetravus
Island x30
[Bot2 Deck]
Island x30
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Tetravus payment redirect probe.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Black Lotus#1.Activate(Color: White)
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Hand.Tetravus.Cast
Bot1.Pass
Bot2.Pass
[T3 Bot1.Upkeep]
Bot1.Decline
Bot1.PayCost(X: 1)
[T5 Bot1.Upkeep]
Bot1.PayCost
Bot1.Choose(Tetravite Token)
Bot1.Decline
[T5 Bot1.Draw]
[Expect]
Bot1.Battlefield.Tetravus.Counters.PlusOnePlusOne = 3
Bot1.Battlefield.Tetravite Token = (absent)
Exile.Count = 0
";
    let script = GameScript::parse(source, "tetravus-redirected-exile-cost").unwrap();
    run_on(&script, table()).unwrap_or_else(|failure| panic!("{}", failure.report));
}
