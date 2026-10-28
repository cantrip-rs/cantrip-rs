use cantrip_api::*;
use cantrip_mechanics::library::exile_top_play_until_end_of_next_turn;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "When this creature enters, exile the top card of your library. Until the end of your next turn, you may play that card."
/// The ability's controller plays it, a land under the land-play rules and a spell with all its costs and timing (CR 305.1,
/// 601.2); an empty library exiles nothing.
fn exile_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_top_play_until_end_of_next_turn(g, item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::PROWESS,
        triggered(
            triggers::THIS_ENTERS,
            exile_top,
            "When this creature enters, exile the top card of your library. Until the end of your next turn, you may play that card.",
        ),
    ],
    ..oracle::CRIMSON_OPERATIVE
};
