use cantrip_api::*;
use cantrip_mechanics::library::exile_top_may_play;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// III: "Exile the top three cards of target opponent's library. You may play those cards this turn." (CR 406, 305.1, 601.3): the
/// controller of the Saga may play them, lands included, until the end of the turn (CR 514.2), whoever owns them; with the target
/// no longer legal nothing is exiled (CR 608.2b).
fn exile_three_from_target_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    exile_top_may_play(g, opponent, 3, item.controller, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, tokens::create_a_treasure, "I \u{2014} Create a Treasure token."),
        chapter(2, tokens::create_a_treasure, "II \u{2014} Create a Treasure token."),
        chapter(3, exile_three_from_target_opponent, "III \u{2014} Exile the top three cards of target opponent's library. You may play those cards this turn.").targets(&[targets::OPPONENT]),
    ],
    ..oracle::KORVOLD_AND_THE_NOBLE_THIEF
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
