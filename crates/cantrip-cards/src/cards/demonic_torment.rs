use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// Prevent combat damage dealt by the enchanted creature (CR 615.1).
fn combat_damage_by_enchanted(q: &dyn Query, asker: &Asker, damage: &DamageEvent) -> bool {
    damage.combat && q.attached_to(asker.source) == Some(damage.source)
}

pub static CARD: CardDef = CardDef {
    preventions: &[PreventionDef {
        hint: "prevents all combat damage that would be dealt by enchanted creature",
        applies: combat_damage_by_enchanted,
        amount: PreventionAmount::All,
    }],
    attack_rules: &[attach::enchanted_creature_cant_attack],
    enchant: Some(targets::CREATURE),
    ..oracle::DEMONIC_TORMENT
};
