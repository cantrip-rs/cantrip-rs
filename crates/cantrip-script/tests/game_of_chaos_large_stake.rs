//! Force 32 flips without a seed or the scenario's ten-face dice limit (CR 705.3).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn all_flips_win(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool { player == asker.controller }
fn any_life(_: &dyn Query, _: &Asker, p: &Proposal) -> bool { matches!(p, Proposal::LifeChange(_)) }
fn observe_then_reduce(q: &dyn Query, _: &Asker, p: &mut Proposal) {
    let Proposal::LifeChange(l) = p else { return };
    let flips = q.history().records.iter().filter(|r| matches!(r.event, Event::CoinFlipped { .. })).count();
    if flips == 32 { assert_eq!(l.magnitude, 2_147_483_648, "the 32nd stake reaches replacement intact"); }
    if flips == 64 { assert_eq!(l.magnitude, 1u64 << 63, "the 64th stake reaches replacement intact"); }
    l.magnitude = 1;
}
static FIXER: CardDef = CardDef {
    name: "Forced stake probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Artifact]), coin_flip_rules: &[all_flips_win],
    replacements: &[replacement(ReplacementKind::LifeChange, any_life, observe_then_reduce, "one life instead")],
    ..CardDef::BLANK
};
#[test]
fn thirty_two_deterministic_flips_present_the_full_stake_to_replacements() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&FIXER])));
    let mut text = String::from(r#"
[Bot1 Deck]
Forced stake probe
Game of Chaos
Black Lotus
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Forced stake probe.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Game of Chaos.Cast(Bot2)
Bot1.Pass
Bot2.Pass
"#);
    for _ in 0..31 { text.push_str("Bot1.Accept\n"); }
    text.push_str("Bot1.Decline\n[Expect]\nBot1.Life = 52\nBot2.Life = -12\n");
    let script = GameScript::parse(&text, "game-of-chaos-large-stake").unwrap();
    let result = run_on(&script, table).unwrap_or_else(|f| panic!("{}", f.report));
    assert_eq!(result.game.log.iter().filter(|r| matches!(r.event, Event::CoinFlipped { .. })).count(), 32);
}

#[test]
fn sixty_fourth_flip_reaches_replacement_before_signed_conversion() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&FIXER])));
    let mut text = String::from(r#"
[Bot1 Deck]
Forced stake probe
Game of Chaos
Black Lotus
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Forced stake probe.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Game of Chaos.Cast(Bot2)
Bot1.Pass
Bot2.Pass
"#);
    for _ in 0..63 { text.push_str("Bot1.Accept\n"); }
    text.push_str("Bot1.Decline\n[Expect]\nBot1.Life = 84\nBot2.Life = -44\n");
    let script = GameScript::parse(&text, "game-of-chaos-64-flips").unwrap();
    let result = run_on(&script, table).unwrap_or_else(|f| panic!("{}", f.report));
    assert_eq!(result.game.log.iter().filter(|r| matches!(r.event, Event::CoinFlipped { .. })).count(), 64);
}
