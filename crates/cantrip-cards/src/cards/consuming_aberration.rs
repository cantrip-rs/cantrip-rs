use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

/// "Consuming Aberration's power and toughness are each equal to the number of cards in your opponents' graveyards" (CR 604.3, 613.4a):
/// a characteristic-defining ability. "Your opponents" are those of the controller still in the game (CR 800.4a); a graveyard holds the
/// cards its owner owns (CR 404.1).
fn power_and_toughness_are_cards_in_opponents_graveyards(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let n: usize = state.opponents(q.controller(source)).map(|p| state.zone(Zone::graveyard(p)).len()).sum();
    chars.power = Some(n as i32);
    chars.toughness = Some(n as i32);
}

/// "Each opponent reveals cards from the top of their library until they reveal a land card, then puts those cards into their
/// graveyard" (CR 701.20, 101.4, 400.3): every opponent reveals, and the cards of all of them, lands included, go to their
/// graveyards together. An opponent with no land in their library reveals, and puts into their graveyard, the whole of it.
fn each_opponent_reveals_until_a_land_then_puts_those_cards_into_their_graveyard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let mut revealed: Vec<ObjectId> = Vec::new();
    for opponent in opponents {
        let (found, rest) = library::reveal_until_n(g, opponent, 1, &|q: &dyn Query, id: ObjectId| q.chars(id).is(CardType::Land));
        revealed.extend(rest);
        revealed.extend(found);
    }
    g.put_into_graveyard(&revealed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_CAST_A_SPELL,
        each_opponent_reveals_until_a_land_then_puts_those_cards_into_their_graveyard,
        "Whenever you cast a spell, each opponent reveals cards from the top of their library until they reveal a land card, then puts those cards into their graveyard.",
    )],
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_and_toughness_are_cards_in_opponents_graveyards,
        "Consuming Aberration's power and toughness are each equal to the number of cards in your opponents' graveyards.",
    )
    .lost_pt(0, 0)],
    ..oracle::CONSUMING_ABERRATION
};
