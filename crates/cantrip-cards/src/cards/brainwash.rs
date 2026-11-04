use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchanted creature can't attack unless its controller pays {3}." (CR 604.1).
fn enchanted_creature_cant_attack_unless_rule(
    q: &dyn Query,
    asker: &Asker,
    attacker: ObjectId,
    _: Target,
) -> Option<ManaCost> {
    (q.state().obj(asker.source).attached_to == Some(attacker)).then(|| mana("{3}"))
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    attack_costs: &[enchanted_creature_cant_attack_unless_rule],
    ..oracle::BRAINWASH
};
