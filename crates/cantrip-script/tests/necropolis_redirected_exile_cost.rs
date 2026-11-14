//! A replacement can redirect an exile cost without undoing the payment (CR 118.11, 614.6).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn graveyard_to_exile(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ZoneChange(z) if z.is_card && z.from.is(ZoneKind::Graveyard) && z.to.is(ZoneKind::Exile))
}
fn hand_instead(q: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ZoneChange(z) = p { z.to = Zone::hand(q.obj(z.object).owner); }
}
static REROUTER: CardDef = CardDef {
    name: "Probe Graveyard Exile Rerouter",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, graveyard_to_exile, hand_instead, "put a card into its owner's hand instead of exiling it from a graveyard")],
    ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&REROUTER])))
}

#[test]
fn redirected_exile_cost_is_paid_but_has_no_exiled_card_to_count() {
    let source = "
[Bot1 Deck]
Necropolis
Black Lotus
Mox Pearl
Mox Sapphire
Mox Jet
Entomb
Probe Graveyard Exile Rerouter
Hill Giant
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Probe Graveyard Exile Rerouter.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Entomb.Cast
Bot1.Search(Hill Giant)
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Necropolis.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Necropolis.Activate
Bot1.Choose(Hill Giant)
? Stack.Count = 1
? Bot1.Hand = Hill Giant
? Exile.Count = 0
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Necropolis.Toughness = 1
Bot1.Hand = Hill Giant
Exile.Count = 0
";
    let script = GameScript::parse(source, "necropolis-redirected-exile-cost").unwrap();
    run_on(&script, table()).unwrap_or_else(|failure| panic!("{}", failure.report));
}
