use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{copy, targets};
use cantrip_oracle as oracle;

/// "Target nonland card with mana value 3 or less from your graveyard" (CR 115.1, 202.3): a card in the ability controller's own
/// graveyard that is not a land card, its mana value read as the game reads it in a graveyard (X is 0, CR 202.3e).
fn nonland_card_with_mana_value_3_or_less_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id)
        if targets::card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[], 3) && !q.chars(id).is(CardType::Land))
}

// "Exile target nonland card with mana value 3 or less from your graveyard. Copy it, then you may cast the copy without paying its
// mana cost" is `copy::exile_first_target_and_may_cast_a_copy_without_paying`.

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, copy::exile_first_target_and_may_cast_a_copy_without_paying, "When Shiko enters, exile target nonland card with mana value 3 or less from your graveyard. Copy it, then you may cast the copy without paying its mana cost.")
        .targets(&[target(nonland_card_with_mana_value_3_or_less_in_your_graveyard)])],
    ..oracle::SHIKO_PARAGON_OF_THE_WAY
};
