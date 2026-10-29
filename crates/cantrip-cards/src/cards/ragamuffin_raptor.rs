use cantrip_api::*;
use cantrip_mechanics::effects::return_first_target_to_hand;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Creature or Food card from your graveyard" (CR 115.1, 205.3): a creature card, or a card with the Food subtype (an artifact,
/// CR 205.3g), in the graveyard of the ability's controller.
fn creature_or_food_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let food = q.words(asker.source).subtype(Subtype::Food);
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if q.chars(id).is(CardType::Creature) || q.chars(id).has_subtype(food))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        return_first_target_to_hand,
        "When this creature enters, return up to one target creature or Food card from your graveyard to your hand.",
    )
    .targets(&[target(creature_or_food_card_in_your_graveyard).optional()])],
    ..oracle::RAGAMUFFIN_RAPTOR
};
