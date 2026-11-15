use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target instant or sorcery card with mana value 4 or less from your graveyard" (CR 115.1, 202.3).
fn instant_or_sorcery_card_with_mana_value_4_or_less_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[CardType::Instant, CardType::Sorcery], 4)
}

/// "When this creature enters, you may cast target instant or sorcery card with mana value 4 or less from your graveyard without
/// paying its mana cost. If that spell would be put into your graveyard, exile it instead" (CR 603.2, 118.9, 608.2g): see
/// `effects::may_cast_first_target_free_exiling_it`.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        effects::may_cast_first_target_free_exiling_it,
        "When this creature enters, you may cast target instant or sorcery card with mana value 4 or less from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead.",
    )
    .targets(&[target(instant_or_sorcery_card_with_mana_value_4_or_less_in_your_graveyard)])],
    ..oracle::DARING_WAVERIDER
};
