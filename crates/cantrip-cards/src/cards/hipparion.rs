use cantrip_api::*;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { block_costs: &[blocking_power_three_or_greater_costs_one], ..oracle::HIPPARION };

/// "This creature can't block creatures with power 3 or greater unless you pay {1}." (CR 509.1d): a cost to declare this
/// creature blocking an attacker whose power is 3 or more as blockers are declared - {1}, once however many such attackers it
/// is declared blocking (the engine pays each ability once for each creature it taxes) - paid by its controller, the
/// defending player, with the rest of the declaration's costs (CR 509.1e-f). A block of a weaker attacker costs nothing, and
/// the cost is never a requirement (CR 509.1c).
fn blocking_power_three_or_greater_costs_one(q: &dyn Query, asker: &Asker, blocker: ObjectId, attacker: ObjectId) -> Option<ManaCost> {
    (blocker == asker.source && q.chars(attacker).power.unwrap_or(0) >= 3).then_some(ManaCost { generic: 1, ..ManaCost::ZERO })
}
