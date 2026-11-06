use cantrip_api::*;
use cantrip_mechanics::triggers::{ward_discard, THIS_ENTERS_OR_ATTACKS};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

const WARD: &str = "Ward\u{2014}Discard a card. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player discards a card.)";

/// "Exile up to one/two target cards from graveyard(s). For each creature card exiled this way, each opponent loses 1 life and you
/// gain 1 life." The targets that are still legal (CR 608.2b) are exiled together, one event for the batch (CR 101.4, 603.10a),
/// and only the cards that arrived in exile count as "exiled this way" (CR 608.2c); each one's type is that of the card in
/// exile. The life is lost and gained once, in the total, which is the same as once per creature card. The front face's one
/// card makes "for each" the same as "if".
fn exile_and_drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = targets::legal_objects(g, item);
    let exiled = g.exile_all(&cards);
    let creatures = exiled.iter().filter(|&&c| g.chars(c).is(CardType::Creature)).count() as i32;
    if creatures > 0 {
        g.each_opponent_loses_life(item.controller,(creatures) as i32);
        g.gain_life(item.controller,(creatures) as i32);
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        ward_discard(WARD),
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            exile_and_drain,
            "Whenever this creature enters or attacks, exile up to two target cards from graveyards. For each creature card exiled this way, each opponent loses 1 life and you gain 1 life.",
        )
        .targets(&[targets::CARD_IN_A_GRAVEYARD.between(0, 2)]),
    ],
    ..oracle::GRAVEYARD_GLUTTON
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward_discard(WARD),
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            exile_and_drain,
            "Whenever this creature enters or attacks, exile up to one target card from a graveyard. If a creature card was exiled this way, each opponent loses 1 life and you gain 1 life.",
        )
        .targets(&[targets::CARD_IN_A_GRAVEYARD.optional()]),
    ],
    back: Some(&BACK),
    ..oracle::GRAVEYARD_TRESPASSER
};
