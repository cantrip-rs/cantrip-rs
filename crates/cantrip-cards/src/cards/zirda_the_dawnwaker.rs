use cantrip_api::*;
use cantrip_mechanics::{companion, effects, targets};
use cantrip_oracle as oracle;

/// "Abilities you activate that aren't mana abilities cost {2} less to activate. This effect can't reduce the mana in
/// that cost to less than one mana" (CR 601.2f, 118.7): the core asks only non-mana abilities, whatever the zone they
/// are activated from. Mana abilities are not priced this way (they are paid as the cost they are part of is), and a special
/// action is no ability (CR 116.1): `Costed::Ability` alone.
fn two_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    if matches!(costed, Costed::Ability(..)) && player == q.controller(source) { CostDelta::less_to_one(2) } else { CostDelta::NONE }
}

/// "Companion - Each permanent card in your starting deck has an activated ability" (CR 702.139a, 602.1): a mana ability is
/// one, a land's by its basic land types too (CR 305.6, the rulings); a special action is not (CR 116.1).
fn permanents_with_activated_abilities(q: &dyn Query, deck: &StartingDeck) -> bool {
    companion::each_permanent_card(q, deck, |card| {
        q.chars(card).abilities().any(|a| q.activated_ability(a).is_some_and(|d| !d.special_action))
    })
}

pub static CARD: CardDef = CardDef {
    companion: Some(permanents_with_activated_abilities),
    abilities: &[ability(cost("{1}, {T}"), effects::first_target_cant_block_this_turn, "{1}, {T}: Target creature can't block this turn.")
        .targets(&[targets::CREATURE])],
    cost_changes: &[two_less],
    ..oracle::ZIRDA_THE_DAWNWAKER
};
