use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target nonland permanent card in your graveyard with mana value less than or equal to the number of permanent cards in your
/// graveyard" (fathomless descent, CR 115.1, 110.4a, 202.3, 207.2c): the target is itself one of the cards counted, and both are judged
/// again as the spell resolves (CR 608.2b).
fn nonland_permanent_card_up_to_permanent_cards_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::nonland_permanent_card_in_your_graveyard(q, asker, t)
        && matches!(t, Target::Object(id) if q.mana_value(id) as usize <= targets::permanent_cards_in_graveyard(q, asker.controller))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(nonland_permanent_card_up_to_permanent_cards_in_your_graveyard)], effects::return_targets_to_battlefield),
    ..oracle::SQUIRMING_EMERGENCE
};
