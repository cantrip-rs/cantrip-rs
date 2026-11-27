//! Library of Leng's questions: the "you may put it on top of your library" asked once for each card one discard discards
//! names the card it is about (`Purpose::AskMove`), and the order of two cards it puts on top of the library together is
//! asked of their owner, as the Library's (CR 401.4).
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

const TWO_ON_TOP: &str = include_str!("../../../scenarios/library-of-leng-two-cards-of-one-discard-go-on-top-in-the-order-chosen.cantrip");

fn handed_over(before: &str, question: &str) -> cantrip_engine::Game {
    let source = TWO_ON_TOP.split(before).next().unwrap().to_owned() + question;
    run(&GameScript::parse(&source, "library of leng prompt").unwrap()).map_err(|f| f.report).unwrap().game
}

#[test]
fn each_question_names_the_card_it_is_about() {
    let game = handed_over("Bot1.Accept\n", "[Handover Bot1.Accept]\n");
    let RequestKind::May { source, purpose: Purpose::AskMove(question, card) } = &game.pending().unwrap().kind else {
        panic!("expected the Library's question: {:?}", game.pending().unwrap().kind)
    };
    assert_eq!(game.name(*source), "Library of Leng");
    assert_eq!(question.ask, "put the discarded card on top of your library instead of into your graveyard?");
    assert_eq!(game.name(*card), "Lightning Bolt", "the card discarded first is asked about first");
}

#[test]
fn two_cards_put_on_top_together_are_ordered_by_their_owner() {
    let game = handed_over("Bot1.Arrange", "[Handover Bot1.Arrange]\n");
    let pending = game.pending().unwrap();
    assert_eq!(pending.player, PlayerId(0));
    let RequestKind::DistributeCards { source, cards, destinations, .. } = &pending.kind else { panic!("expected the order: {:?}", pending.kind) };
    assert_eq!(game.name(*source), "Library of Leng");
    let names: Vec<&str> = cards.iter().map(|&c| game.name(c)).collect();
    assert_eq!(names, ["Island", "Lightning Bolt"], "top first, as they arrived");
    assert_eq!(destinations.len(), 1);
    assert_eq!((destinations[0].kind, destinations[0].min, destinations[0].max), (DestinationKind::LibraryTop, 2, 2));
}
