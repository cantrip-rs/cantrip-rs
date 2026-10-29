use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

const NAME: &str = "Page, Loose Leaf";

/// "Discard another card named Page, Loose Leaf" (CR 701.9c): a card of the activator's hand with that name. The name it has
/// now (CR 201.2a). This permanent is on the battlefield, so any card of the hand is another.
fn card_named_page(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    library::object_names(q, card).contains(&NAME)
}

fn is_instant_or_sorcery(q: &dyn Query, card: ObjectId) -> bool {
    q.chars(card).is(CardType::Instant) || q.chars(card).is(CardType::Sorcery)
}

/// "Reveal cards from the top of your library until you reveal an instant or sorcery card. Put that card into your hand and
/// the rest on the bottom of your library in a random order." A library holding no such card is revealed whole and put back
/// in a random order (CR 701.20, 401.4).
fn reveal_until_instant_or_sorcery(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let (found, rest) = library::reveal_until(g, you, &is_instant_or_sorcery);
    if let Some(card) = found {
        g.return_to_hand_revealed(&[card]);
    }
    library::rest_to_bottom_in_random_order(g, you, &rest);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            discard(1, card_named_page),
            reveal_until_instant_or_sorcery,
            "Grandeur \u{2014} Discard another card named Page, Loose Leaf: Reveal cards from the top of your library until you reveal an instant or sorcery card. Put that card into your hand and the rest on the bottom of your library in a random order.",
        ),
    ],
    ..oracle::PAGE_LOOSE_LEAF
};
