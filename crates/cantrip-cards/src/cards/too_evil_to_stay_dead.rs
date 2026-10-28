use cantrip_api::*;
use cantrip_mechanics::{effects, targets, teamwork};
use cantrip_oracle as oracle;

/// "Target creature card in your graveyard with mana value 4 or less", or, if the spell was cast using teamwork, "target
/// creature card in your graveyard" instead. The teamwork cost is announced before the target is chosen (CR 601.2b-c,
/// 702.194b), so the target is judged by it, as the spell is cast and again as it resolves (CR 608.2b).
fn creature_card_in_your_graveyard_with_mana_value_4_or_less_or_any_with_teamwork(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    if q.obj(asker.source).paid > 0 {
        (targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.filter)(q, asker, t)
    } else {
        targets::card_in_your_graveyard_with_mana_value_at_most(q, asker, t, &[CardType::Creature], 4)
    }
}

/// "Choose target creature card in your graveyard ... Return the chosen card to the battlefield" (CR 400.7, 608.2b): one target
/// either way, put onto the battlefield under the spell's controller, who owns every card in their own graveyard; nothing if it is no
/// longer there (the spell does not resolve).
/// Teamwork 4 (as an additional cost to cast this spell, you may tap any number of creatures you control with total power 4 or
/// more): an optional cost announced before the target, which is chosen after it.
pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_card_in_your_graveyard_with_mana_value_4_or_less_or_any_with_teamwork)], effects::return_first_target_to_battlefield),
    late_targets: true,
    optional_cost: Some(teamwork::optional(4)),
    labels: &["Cast without teamwork", "Teamwork 4: Tap any number of creatures you control with total power 4 or more"],
    ..oracle::TOO_EVIL_TO_STAY_DEAD
};
