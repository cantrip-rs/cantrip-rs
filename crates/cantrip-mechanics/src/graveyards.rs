//! "A card in a graveyard", any graveyard (CR 404.1): every player's graveyard is read, in seat order and each in the order
//! the zone keeps it (the order its cards arrived, the top last).

use cantrip_api::*;

/// Every card in every player's graveyard: Breaking // Entering's "a creature card from a graveyard", Superior Spider-Man's "any
/// creature card in a graveyard", Echoing Deeps's "a land card in a graveyard" and Necrotic Ooze's "creature cards in all
/// graveyards". The caller narrows them by card type.
pub fn cards_in_graveyards(s: &GameState) -> impl Iterator<Item = ObjectId> + '_ {
    s.seats().flat_map(|p| s.zone(Zone::graveyard(p)).iter().copied())
}

/// Every creature card in every graveyard (CR 404.1), by the characteristics it has there now: Lhurgoyf's and Cruel
/// Somnophage's "the number of creature cards in all graveyards" (as `creature_cards_in_graveyards_in_layers`), Rise of the Dark
/// Realms' "all creature cards from all graveyards" and Superior Spider-Man's "any creature card in a graveyard".
pub fn creature_cards_in_graveyards(q: &dyn Query) -> impl Iterator<Item = ObjectId> + '_ {
    creature_cards(q.state(), move |id| q.chars(id))
}

/// `creature_cards_in_graveyards` during a continuous effect, by the characteristics of the preceding layers: a
/// characteristic-defining ability's count (CR 604.3), Lhurgoyf's and Cruel Somnophage's.
pub fn creature_cards_in_graveyards_in_layers(q: &dyn LayerQuery) -> impl Iterator<Item = ObjectId> + '_ {
    creature_cards(q.state(), move |id| q.chars(id))
}

fn creature_cards<'a>(s: &'a GameState, chars: impl Fn(ObjectId) -> Chars + 'a) -> impl Iterator<Item = ObjectId> + 'a {
    cards_in_graveyards(s).filter(move |&card| chars(card).is(CardType::Creature))
}

/// Whether the card in a graveyard has a quality: the card itself, never the face it would be cast as, since a card has only its
/// normal characteristics there (CR 715.4: Bonecrusher Giant is a creature card, though Stomp is an instant), and a split card has
/// both halves' (CR 709.4: Leave // Chance is an instant and sorcery card). Iroh, Grand Lotus and Lier, Disciple of the Drowned
/// ask it for "instant and sorcery card in your graveyard".
pub fn card_is(q: &dyn Query, card: ObjectId, quality: impl Fn(&CardDef) -> bool) -> bool {
    let printed = q.card(q.obj(card).card);
    match printed.split {
        Some((left, right)) => quality(left) || quality(right),
        None => quality(printed),
    }
}

/// An instant or sorcery card (the `quality` of `card_is`).
pub fn instant_or_sorcery(def: &CardDef) -> bool {
    def.is(CardType::Instant) || def.is(CardType::Sorcery)
}
