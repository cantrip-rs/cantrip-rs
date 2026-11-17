use cantrip_api::*;
use cantrip_mechanics::select::{self, Selection};
use cantrip_mechanics::{choose, effects, triggers};
use cantrip_oracle as oracle;

/// "An artifact or creature card" in the hand, which the player may leave unexiled.
const ARTIFACT_OR_CREATURE_CARD: Selection = Selection::of(select::up_to(1), &[select::ARTIFACT_OR_CREATURE]).called("artifact or creature");

/// "Draw a card. Then you may exile an artifact or creature card from your hand. If you do, create a token that's a copy of the
/// exiled card, except it's a 3/3 Golem artifact creature in addition to its other types." (CR 121.1, 406.3, 707.2, 707.9b, 111.1):
/// the card is exiled from the hand, and the token copies it as it was there (its printed values), with the base power and
/// toughness 3/3 and the Artifact and Creature types and the Golem type added to its own. "Golem" is a word of this card's text
/// (CR 612). No token is made unless a card was exiled.
fn draw_then_may_exile_to_make_a_golem_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    effects::draw(g, item, 1)?;
    g.end_sentence();
    let hand = g.state().zone(Zone::hand(you)).to_vec();
    let Some(&card) = choose::select(g, you, item.object, Purpose::Exile, hand, ARTIFACT_OR_CREATURE_CARD)?.first() else { return Ok(()) };
    let golem = g.words(item.object).subtype(Subtype::Golem);
    let values = g.copiable_values(card).sized(3, 3).with_subtype(golem).with_type(CardType::Artifact).with_type(CardType::Creature);
    let exiled = g.exile_all(&[card]);
    g.end_sentence();
    g.create_token_copies_of(values, you, exiled.len() as u32)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::BEGINNING_OF_YOUR_COMBAT,
        draw_then_may_exile_to_make_a_golem_copy,
        "At the beginning of combat on your turn, draw a card. Then you may exile an artifact or creature card from your hand. If you do, create a token that's a copy of the exiled card, except it's a 3/3 Golem artifact creature in addition to its other types.",
    )],
    ..oracle::NEXUS_OF_BECOMING
};
