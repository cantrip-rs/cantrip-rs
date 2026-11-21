use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, THIS_DIES};
use cantrip_mechanics::{effects, learn};
use cantrip_oracle as oracle;

/// "This creature's power is equal to the number of noncreature, nonland cards in your graveyard" (CR 604.3, layer 7a): a
/// characteristic-defining ability, so it applies in every zone and counts only its controller's graveyard as the layers are
/// worked out, by the characteristics of the preceding layers; a card the graveyard holds is only the card it is there.
fn power_is_noncreature_nonland_cards_in_your_graveyard(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let graveyard = q.state().zone(Zone::graveyard(q.state().obj(source).controller));
    chars.power = Some(graveyard.iter().filter(|&&card| !q.chars(card).is(CardType::Creature) && !q.chars(card).is(CardType::Land)).count() as i32);
}

/// "If there's a Lesson card in your graveyard" (CR 603.4): checked as it dies, with the graveyard it died into, and again as the
/// ability resolves.
fn a_lesson_is_in_your_graveyard(q: &dyn Query, asker: &Asker) -> bool {
    learn::lesson_cards_in_graveyard(q, asker.controller) > 0
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(Layer::PtDefine, power_is_noncreature_nonland_cards_in_your_graveyard, "This creature's power is equal to the number of noncreature, nonland cards in your graveyard.").lost_pt(0, 3)],
    abilities: &[
        ward::<1>("Ward {1} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {1}.)"),
        triggered(THIS_DIES, |g, item| effects::draw(g, item, 1), "When this creature dies, if there's a Lesson card in your graveyard, draw a card.").condition(a_lesson_is_in_your_graveyard),
    ],
    ..oracle::DRAGONFLY_SWARM
};
