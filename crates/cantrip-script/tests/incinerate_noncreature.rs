//! A damaged planeswalker isn't a creature dealt damage this way (CR 611.2c).
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn incinerate_does_not_restrict_a_damaged_noncreature() {
    // Cast Gideon normally so its loyalty counters are put by the actual entry procedure.
    let script = GameScript::parse(r#"
[Bot1 Deck]
Forest
Black Lotus
Mox Pearl
Mox Ruby
Gideon, Ally of Zendikar
Incinerate
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Forest.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Gideon, Ally of Zendikar.Cast
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Forest.Activate
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Incinerate.Cast(Bot1.Battlefield.Gideon, Ally of Zendikar)
Bot1.Pass
Bot2.Pass
[T1 Bot1.Main2]
[Expect]
Bot1.Battlefield.Gideon, Ally of Zendikar.Counters.Loyalty = 1
"#, "incinerate-noncreature").unwrap();
    let game = run(&script).unwrap().game;
    let walker = game.state.battlefield().iter().copied().find(|&id| game.name(id) == "Gideon, Ally of Zendikar").unwrap();
    assert!(game.log.iter().any(|r| matches!(r.event, Event::CountersPut { object, kind: CounterKind::Loyalty, amount: 4, total: 4, .. } if object == walker)), "Gideon entered with four loyalty counters");
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Damage { target: Target::Object(id), amount: 3, .. } if id == walker)), "Incinerate dealt its three damage");
    assert_eq!(game.state.obj(walker).counters(CounterKind::Loyalty), 1);
    assert!(!game.state.effects.iter().any(|e| e.affected == walker && e.kind == EffectKind::CantBeRegenerated), "a noncreature gets no regeneration restriction");
}
