use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Can't be blocked by creatures with power 2 or less" (CR 509.1b): asked of each possible block while the effect lasts, the blocker's
/// power as blockers are declared; a block already made stands if powers change afterwards (the rulings), and a creature with no
/// power can't block it either.
const NO_SMALL_BLOCKERS: LastingBlockRule = LastingBlockRule {
    text: "Can't be blocked by creatures with power 2 or less",
    allows: |q, on, blocker, attacker| on != attacker || q.chars(blocker).power.is_some_and(|power| power > 2),
};

/// "This creature can't be blocked by creatures with power 2 or less this turn." A resolved effect on this creature, not an ability it
/// gains (CR 113.1a, 611.2c), so it lasts the turn whatever abilities the creature loses afterwards (CR 613.1f) and ends if it leaves
/// the battlefield (CR 400.7). Activating it after blockers are declared undoes no block (the rulings).
fn cant_be_blocked_by_power_two_or_less_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::BlockRestriction { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::scry_two, "When this creature enters, scry 2."),
        ability(cost("{3}{G}"), cant_be_blocked_by_power_two_or_less_this_turn, "{3}{G}: This creature can't be blocked by creatures with power 2 or less this turn."),
    ],
    lasting_block_rules: &[NO_SMALL_BLOCKERS],
    ..oracle::CAVERN_STOMPER
};
