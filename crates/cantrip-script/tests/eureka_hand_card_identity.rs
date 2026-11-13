//! Eureka must not offer a token temporarily in a hand during resolution (CR 108.2, 111.6, 704.5d).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

// This test-only entry action forces a token into the next player's hand during Eureka's resolution,
// before state-based actions can remove it. It is a copy of a normal permanent, so its types qualify.
fn token_to_hand(g: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    let template = g.state().battlefield().iter().copied().find(|&id| g.card(g.obj(id).card).name == "Mox Pearl").unwrap();
    let token = g.create_token_copies(template, PlayerId(1), 1)?;
    g.return_to_hand(&token);
    let _ = entering;
    Ok(())
}
static PROBE: CardDef = CardDef {
    name: "Eureka Test Entry", oracle: true, has_cost: true,
    types: TypeSet::of(&[CardType::Artifact]), enters: Some(token_to_hand), ..CardDef::BLANK
};

#[test]
fn a_token_in_hand_is_not_a_card_choice() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&PROBE])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Eureka
Black Lotus
Mox Pearl
Eureka Test Entry
Forest x20
[Bot2 Deck]
Forest
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Eureka.Cast
Bot1.Pass
Bot1.Choose(Eureka Test Entry)
[Handover Bot2.Choose]
"#, "eureka-hand-card-identity").unwrap();
    let result = run_on(&script, table).map_err(|f| f.report).unwrap();
    let game = result.game;
    let request = game.pending().expect("second player's choice");
    let RequestKind::ChooseCards { cards, .. } = &request.kind else { panic!("expected card choice: {request:?}") };
    let hand = game.state.zone(Zone::hand(PlayerId(1)));
    let token = hand.iter().copied().find(|&id| game.state.obj(id).token).expect("token remains in hand during resolution");
    assert!(!cards.contains(&token), "a token is not a card");
    assert!(cards.iter().any(|&id| game.cards.get(game.state.obj(id).card).name == "Forest"), "a real Forest is still offered");
}
