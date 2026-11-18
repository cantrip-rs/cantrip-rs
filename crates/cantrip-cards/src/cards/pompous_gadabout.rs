use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "This creature can't be blocked by creatures with no name" (CR 509.1b, 201.2): a face-down creature has no name (CR 708.2a),
/// so a blocker whose name is empty cannot block; a token has the name its effect gave it, so it can.
fn cant_be_blocked_by_creatures_with_no_name(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || !q.chars(blocker).name.is_empty()
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, statics::itself_during_your_turn, statics::have_hexproof, "During your turn, this creature has hexproof.")],
    block_rules: &[cant_be_blocked_by_creatures_with_no_name],
    ..oracle::POMPOUS_GADABOUT
};
