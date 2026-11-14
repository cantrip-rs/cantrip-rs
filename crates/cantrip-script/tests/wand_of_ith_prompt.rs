use cantrip_engine::*;
use cantrip_script::{GameScript, run};

fn check(source: &str, life_cost: u32) {
    let script = source.split("Bot2.PayCost").next().unwrap().to_owned() + "[Handover Bot2.Accept]\n";
    let game = run(&GameScript::parse(&script, "wand payment prompt").unwrap()).map_err(|f| f.report).unwrap().game;
    let RequestKind::UnlessPays { source, subject, life, question, .. } = &game.pending().unwrap().kind else { panic!("expected payment choice") };
    assert_eq!(*life, life_cost);
    assert_ne!(source, subject);
    assert!(game.state.zone(Zone::hand(PlayerId(1))).contains(subject));
    assert!(game.log.iter().any(|r| matches!(r.event, Event::Revealed { player: PlayerId(1), card } if card == *subject)));
    assert_eq!(question.ask, "pay {cost}, or discard {subject}?");
    assert_eq!(question.no, "Discard {subject}");
}

#[test]
fn land_payment_names_the_revealed_card() {
    check(include_str!("../../../scenarios/wand-of-ith-land-payment-and-timing.cantrip"), 1);
}

#[test]
fn nonland_payment_names_the_revealed_card() {
    check(include_str!("../../../scenarios/wand-of-ith-nonland-payment.cantrip"), 4);
}
