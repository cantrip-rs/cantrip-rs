//! Yawgmoth's Testament can replace Necropotence's exile with a move to the library (CR 614.6).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn applies(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.is_card && z.from.is(ZoneKind::Library) && z.to.is(ZoneKind::Exile))
}

fn replace(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.to = Zone::library(q.obj(z.object).owner);
    }
}

static REPLACEMENT: CardDef = CardDef {
    name: "Probe Testament", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, applies, replace, "puts a card into its library instead")],
    ..CardDef::BLANK
};

#[test]
fn replaced_exile_is_not_put_into_hand() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&REPLACEMENT])));
    let text = r#"
[Bot1 Deck]
Probe Testament
Black Lotus
Necropotence
Swamp x4
Forest
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Probe Testament.Cast
Bot1.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Necropotence.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Necropotence.Activate
Bot1.Pass
Bot2.Pass
[T1 Bot1.End]
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Hand.Count = 4
Bot1.Library.Count = 21
Exile.Count = 0
Bot1.Life = 19
"#;
    let script = GameScript::parse(text, "necropotence-replaced-exile").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|failure| failure.report).unwrap_or_default());
}
