use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "They are Food artifacts with '{2}, {T}, Sacrifice this artifact: You gain 3 life.' (They lose all other types and
/// subtypes.)": what they enter as (CR 614.12, 613.1d, 613.1f), never a creature. Supertypes stay, so a legendary creature is
/// a legendary Food; the ability is the first `delayed` one.
static AS_FOOD: EntersAs = EntersAs {
    types: Some(TypeSet::of(&[CardType::Artifact])),
    loses_subtypes_of: &[CardType::Artifact],
    subtypes: &[Subtype::Food],
    abilities: &[0],
    ..EntersAs::NONE
};

/// "Put onto the battlefield under your control all creature cards in your opponents' graveyards that were put there from
/// the battlefield this turn. They are Food artifacts with '{2}, {T}, Sacrifice this artifact: You gain 3 life.' (They lose
/// all other types and subtypes.)" (CR 400.7, 614.12, 613.1d, 613.1f).
///
/// The cards are the creature cards still in an opponent's graveyard (a token is never a card there, and one that left and
/// came back is a new object) whose move into that graveyard, from the battlefield, was this turn's. They enter together
/// under the spell's controller, as Food artifacts and not as creatures: nothing that watches creatures enter sees them, and
/// what watches artifacts does. It has no end, and it is about those objects alone (CR 400.7).
fn put_onto_the_battlefield_as_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    let mut cards = Vec::new();
    for opponent in opponents {
        for card in history::put_into_graveyard_from_battlefield_this_turn(g, opponent) {
            if g.chars(card).is(CardType::Creature) {
                cards.push((card, you));
            }
        }
    }
    g.put_all_onto_battlefield_becoming(&cards, item, &AS_FOOD, &[])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], put_onto_the_battlefield_as_food),
    delayed: &[ability(cost("{2}, {T}").and(SACRIFICE_THIS), |g, item| effects::gain_life(g, item, 3), "{2}, {T}, Sacrifice this artifact: You gain 3 life.")],
    ..oracle::SUPPER_FOR_SPIDERS
};
