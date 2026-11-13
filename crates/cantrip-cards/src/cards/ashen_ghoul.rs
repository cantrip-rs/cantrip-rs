use cantrip_api::*;
use cantrip_oracle as oracle;

use cantrip_mechanics::effects::return_this_card_to_battlefield;

/// "{B}: Return this card from your graveyard to the battlefield. Activate only during your upkeep and only if three or more creature cards are above this card." (CR 602.5).
fn return_card_graveyard_battlefield_activate_if(q: &dyn Query, asker: &Asker) -> bool {
    Some(asker.controller)
        .is_some_and(|player: PlayerId| {
            q.state().turn.active == player && q.state().turn.step() == Step::Upkeep
        })
        && Some(asker.controller)
            .zip(Some(asker.source))
            .is_some_and(|(p, card): (PlayerId, ObjectId)| {
                let cards = q.state().zone(Zone::graveyard(p));
                cards
                    .iter()
                    .position(|&o| o == card)
                    .is_some_and(|at| {
                        (cards[at + 1..]
                            .iter()
                            .copied()
                            .filter(|&id| q.chars(id).is(CardType::Creature))
                            .count() as i64) >= 3
                    })
            })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{B}"),
                return_this_card_to_battlefield,
                "{B}: Return this card from your graveyard to the battlefield. Activate only during your upkeep and only if three or more creature cards are above this card.",
            )
            .condition(return_card_graveyard_battlefield_activate_if)
            .from(ZoneKind::Graveyard),
    ],
    ..oracle::ASHEN_GHOUL
};
