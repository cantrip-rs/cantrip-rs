use cantrip_api::*;
use cantrip_mechanics::start_your_engines::has_max_speed;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Hazoret can't attack ... unless you have max speed" (CR 508.1c, 702.179e): asked of this creature as attackers are declared,
/// by its controller's speed then; once declared it stays attacking whatever happens to the speed (CR 506.4).
fn attacks_only_at_max_speed(q: &dyn Query, asker: &Asker, attacker: ObjectId, _: Target) -> bool {
    asker.source != attacker || has_max_speed(q.state(), q.controller(attacker))
}

/// "Hazoret can't ... block unless you have max speed" (CR 509.1b, 702.179e): asked as blockers are declared.
fn blocks_only_at_max_speed(q: &dyn Query, source: ObjectId, blocker: ObjectId, _: ObjectId) -> bool {
    source != blocker || has_max_speed(q.state(), q.controller(blocker))
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[ability(cost("{1}").and(TAP), effects::target_creature_cant_be_blocked_this_turn, "{1}, {T}: Target creature with power 2 or less can't be blocked this turn.")
        .targets(&[targets::CREATURE_WITH_POWER_2_OR_LESS])],
    attack_rules: &[attacks_only_at_max_speed],
    block_rules: &[blocks_only_at_max_speed],
    ..oracle::HAZORET_GODSEEKER
};
