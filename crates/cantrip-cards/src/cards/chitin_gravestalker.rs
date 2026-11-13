use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast for each artifact and/or creature card in your graveyard" (CR 601.2f, 118.7a): counted as the
/// cost is determined, the cards as they are in the caster's graveyard; an artifact creature card counts once (the ruling). The
/// reduction never takes off a colored symbol (CR 118.7a), which the core leaves in place, so the cost is never below {B}.
fn less_per_artifact_or_creature_card_in_your_graveyard(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let cards = q.state().zone(Zone::graveyard(player)).iter().filter(|&&id| q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Creature)).count();
    CostDelta::less(cards as u32)
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_per_artifact_or_creature_card_in_your_graveyard],
    abilities: &[triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::CHITIN_GRAVESTALKER
};
