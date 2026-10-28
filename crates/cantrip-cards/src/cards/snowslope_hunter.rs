use cantrip_api::*;
use cantrip_mechanics::library::exile_top_card_play_until_end_of_next_turn;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::your_turn;
use cantrip_oracle as oracle;

/// "Sacrifice another creature or artifact: Exile the top card of your library. You may play it until the end of your next
/// turn. Activate only during your turn and only once each turn" (CR 602.5b): the sacrifice is a cost (CR 118.3), so nothing
/// happens to the library if it cannot be paid; "once each turn" counts this permanent's own activations.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        sacrifice(1, filters::another_creature_or_artifact),
        exile_top_card_play_until_end_of_next_turn,
        "Sacrifice another creature or artifact: Exile the top card of your library. You may play it until the end of your next turn. Activate only during your turn and only once each turn.",
    )
    .condition(your_turn)
    .once_each_turn()],
    ..oracle::SNOWSLOPE_HUNTER
};
