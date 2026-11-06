//! Model CR 903.9b's optional commander-to-command-zone replacement as a deterministic effect:
//! one card headed from hand to library goes elsewhere, so it was not put on the library.
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn artifact_from_hand_to_library(q: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ZoneChange(z) if z.from.is(ZoneKind::Hand) && z.to.is(ZoneKind::Library) && q.chars(z.object).is(CardType::Artifact))
}
fn command_instead(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ZoneChange(z) = p { z.to = Zone::COMMAND; }
}
static REROUTER: CardDef = CardDef {
    name: "Probe Commander Replacement", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, artifact_from_hand_to_library, command_instead, "put the artifact card into the command zone instead")],
    ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&REROUTER])))
}
#[test]
fn mask_counts_only_cards_put_on_library() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Probe Commander Replacement
Black Lotus
Mox Pearl
Mox Sapphire
Mox Ruby
Jester's Mask
Island
Island x20
[Bot2 Deck]
Memnite
Mountain x6
Forest x20
[Actions]
Bot1.Hand.Probe Commander Replacement.Cast
Bot1.Pass
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Jester's Mask.Cast
Bot1.Pass
Bot2.Pass
[T3 Bot1.Main1]
Bot1.Hand.Island.Play
Bot1.Battlefield.Island.Activate
Bot1.Battlefield.Jester's Mask.Activate(Bot2)
Bot1.Pass
Bot2.Pass
Bot2.Arrange(Memnite; Mountain#1; Mountain#2; Mountain#3; Mountain#4; Mountain#5; Mountain#6)
! Bot1.Search(Mountain; Forest; Forest; Forest; Forest; Forest; Forest)
Bot1.Search(Mountain; Forest; Forest; Forest; Forest; Forest)
[Expect]
Bot2.Hand.Count = 6
Bot2.Shuffles = 1
"#, "mask-redirect").unwrap();
    let result = run_on(&script, table());
    assert!(result.is_ok(), "{}", result.err().map_or_else(String::new, |f| f.report));
}
