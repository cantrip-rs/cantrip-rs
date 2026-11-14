use cantrip_api::*;
use cantrip_mechanics::ferocious;
use cantrip_oracle as oracle;

/// "You control another creature with power 4 or greater": asked of the creature's controller as it is now.
fn controls_another_big_creature(q: &dyn Query, this: ObjectId) -> bool {
    ferocious::you_control_another(q, q.controller(this), this)
}

/// "This creature can't attack ... unless you control another creature with power 4 or greater" (CR 508.1c): asked of this
/// creature as attackers are declared; once declared it stays attacking whatever happens to the other creature (CR 506.4).
fn attacks_only_with_another_big_creature(q: &dyn Query, asker: &Asker, attacker: ObjectId, _: Target) -> bool {
    asker.source != attacker || controls_another_big_creature(q, attacker)
}

/// "This creature can't ... block unless you control another creature with power 4 or greater" (CR 509.1b).
fn blocks_only_with_another_big_creature(q: &dyn Query, source: ObjectId, blocker: ObjectId, _: ObjectId) -> bool {
    source != blocker || controls_another_big_creature(q, blocker)
}

pub static CARD: CardDef = CardDef {
    attack_rules: &[attacks_only_with_another_big_creature],
    block_rules: &[blocks_only_with_another_big_creature],
    ..oracle::TIGER_DILLO
};
