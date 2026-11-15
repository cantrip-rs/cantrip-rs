use cantrip_api::*;
use cantrip_mechanics::{gift, targets, tokens};
use cantrip_oracle as oracle;

/// "Target creature an opponent controls", or, if the gift was promised, "target nonland permanent an opponent controls"
/// instead. The promise is announced before the target is chosen (CR 601.2b-c, 702.174m), so the target is judged by it,
/// and again as the spell resolves (CR 608.2b).
fn creature_or_nonland_permanent(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    gift::target_by_promise(q, asker, t, &targets::NONLAND_PERMANENT_AN_OPPONENT_CONTROLS, &targets::CREATURE_AN_OPPONENT_CONTROLS)
}

/// "Gift a tapped Fish" (CR 702.174f): the Fish is the card's own token.
pub static EXTRA: &[&CardDef] = &[&tokens::FISH];

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_or_nonland_permanent)], cantrip_mechanics::effects::return_first_target_to_hand),
    gift: Some(gift::TAPPED_FISH),
    ..oracle::INTO_THE_FLOOD_MAW
};
