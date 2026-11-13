use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Exile up to two target cards from a single graveyard. If at least one creature card was exiled this way, each opponent loses 2
/// life and you gain 2 life." The targets still legal (CR 608.2b) are exiled together (CR 101.4, 701.13); only cards that arrived
/// in exile count as "exiled this way" (CR 608.2c). One creature card or two, the life is lost and gained once.
fn exile_and_drain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exile_all(&targets::legal_objects(g, item));
    if exiled.iter().any(|&c| g.chars(c).is(CardType::Creature)) {
        g.end_sentence();
        effects::each_opponent_loses_life_you_gain_life::<2>(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        exile_and_drain,
        "When this creature enters, exile up to two target cards from a single graveyard. If at least one creature card was exiled this way, each opponent loses 2 life and you gain 2 life.",
    )
    .targets(targets::UP_TO_TWO_CARDS_FROM_A_SINGLE_GRAVEYARD)],
    ..oracle::SOUL_SHACKLED_ZOMBIE
};
