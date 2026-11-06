use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Target an enchantment card in your graveyard (CR 115.1c, 404.1).
fn enchantment_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_graveyard(q, t, &[CardType::Enchantment])
        && matches!(t, Target::Object(id) if q.obj(id).zone == Zone::graveyard(asker.controller) && !q.obj(id).token)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{5}").and(TAP), effects::return_first_target_to_hand,
        "{5}, {T}: Return target enchantment card from your graveyard to your hand.")
        .targets(&[target(enchantment_in_your_graveyard)])],
    ..oracle::SKULL_OF_ORM
};
