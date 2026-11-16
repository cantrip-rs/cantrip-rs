use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "As long as this creature is untapped, creatures can't attack you or planeswalkers you control unless their controller pays
/// {1} for each of those creatures." (CR 508.1d, 508.1h, 604.1): the shared tax on attacking its controller or their
/// planeswalkers, only while the source is untapped, judged as attackers are declared.
fn attacking_you_or_your_planeswalkers_costs_one_while_untapped(q: &dyn Query, asker: &Asker, attacker: ObjectId, target: Target) -> Option<ManaCost> {
    if q.state().obj(asker.source).tapped {
        return None;
    }
    statics::attacking_you_or_your_planeswalkers_costs::<1>(q, asker, attacker, target)
}

/// "As long as this creature is attacking, creatures can't block unless their controller pays {1} for each of those creatures."
/// (CR 509.1d, 509.1e): a cost to declare any creature blocking any attacker while the source is an attacking creature,
/// {1} for each creature however many attackers it blocks, paid by that creature's controller. It is never a requirement
/// (CR 509.1c).
fn blocking_costs_one_while_this_is_attacking(q: &dyn Query, asker: &Asker, _blocker: ObjectId, _attacker: ObjectId) -> Option<ManaCost> {
    q.is_attacking(asker.source).then_some(ManaCost { generic: 1, ..ManaCost::ZERO })
}

pub static CARD: CardDef = CardDef {
    attack_costs: &[attacking_you_or_your_planeswalkers_costs_one_while_untapped],
    block_costs: &[blocking_costs_one_while_this_is_attacking],
    ..oracle::ARCHANGEL_OF_TITHES
};
pub static EXTRA: &[&CardDef] = &[];
