//! Ob Nixilis, Captive Kingpin's one-or-more-opponents trigger sees Pox's losses together.
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};
use std::sync::OnceLock;

fn opponent_lost_one(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::LifeChange { player, delta: -1 } if player != asker.controller => Fired::about(Target::Player(player)),
        _ => None,
    }
}
fn count_trigger(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}
// The printed trigger, with a life gain standing in for its counter and exile, makes its count observable.
static WATCHER: CardDef = CardDef {
    name: "Pox life observer", oracle: true,
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when_asked(EventMask::LIFE_CHANGE, opponent_lost_one), count_trigger,
        "Whenever one or more opponents each lose exactly 1 life, gain 1 life.").once_per_batch()],
    ..CardDef::BLANK
};

#[test]
fn opponents_losing_one_life_trigger_once_together() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| { let mut cards = cantrip_cards::all(); cards.push(&WATCHER); CardTable::new(cards) });
    let text = r#"
[Life]
Bot2: 3
Bot3: 3
[Bot1 Deck]
Pox life observer
Black Lotus
Pox
Island x20
[Bot2 Deck]
Island x20
[Bot3 Deck]
Island x20
[Actions]
Bot1.Hand.Pox life observer.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Pox.Cast
Bot1.Pass
Bot2.Pass
Bot3.Pass
[Expect]
Bot1.Life = 14
Bot2.Life = 2
Bot3.Life = 2
"#;
    let script = GameScript::parse(text, "Pox simultaneous life").unwrap();
    if let Err(failure) = run_on(&script, table) { panic!("{}", failure.report); }
}
