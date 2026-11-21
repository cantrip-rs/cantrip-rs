use cantrip_api::*;
use cantrip_mechanics::{effects, targets, your_turn};
use cantrip_oracle as oracle;

/// "{T}, Discard a card: Draw a card." The discard is a cost (CR 602.2); the draw is the effect (CR 608.2).
/// "{1}, {T}, Sacrifice Professor Zei: Return target instant or sorcery card from your graveyard to your hand. Activate only
/// during your turn." (CR 602.5b): the card is a target (CR 115.1), gone from the graveyard by the time the ability resolves it
/// is not returned (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP.and(discard_cards(1)), |g, item| effects::draw(g, item, 1), "{T}, Discard a card: Draw a card."),
        ability(
            cost("{1}").and(TAP).and(SACRIFICE_THIS),
            effects::return_each_target_to_hand,
            "{1}, {T}, Sacrifice Professor Zei: Return target instant or sorcery card from your graveyard to your hand. Activate only during your turn.",
        )
        .targets(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD])
        .condition(your_turn),
    ],
    ..oracle::PROFESSOR_ZEI_ANTHROPOLOGIST
};
