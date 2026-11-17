use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{copy, history, targets};
use cantrip_oracle as oracle;

/// "Target instant or sorcery card with mana value 2 or less from your graveyard" (CR 115.1, 202.3): a card in the ability
/// controller's own graveyard, its mana value read as the game reads it there (X is 0, CR 202.3e).
fn instant_or_sorcery_card_with_mana_value_2_or_less_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[CardType::Instant, CardType::Sorcery], 2)
}

/// Void is an ability word (CR 207.2c): the intervening "if" (CR 603.4) is `history::void`, checked as the creature enters
/// and again as the ability resolves. "Up to one target" may be none: then, or with the target gone, nothing is exiled or copied
/// (CR 608.2b); `copy::exile_first_target_and_may_cast_a_copy_without_paying` does the rest.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        copy::exile_first_target_and_may_cast_a_copy_without_paying,
        "Void \u{2014} When this creature enters, if a nonland permanent left the battlefield this turn or a spell was warped this turn, exile up to one target instant or sorcery card with mana value 2 or less from your graveyard. Copy it. You may cast the copy without paying its mana cost.",
    )
    .condition(|q, _| history::void(q))
    .targets(&[target(instant_or_sorcery_card_with_mana_value_2_or_less_in_your_graveyard).optional()])],
    ..oracle::ROVING_ACTUATOR
};
