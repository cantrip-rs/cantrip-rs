use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Immune — Prevent all combat damage that would be dealt to Diamond Weapon." (CR 615.1, 510.2): applies to combat damage
/// dealt to this object by anything; the prevention is not considered as a trampling attacker assigns its damage (CR 702.19b), so
/// it must still assign lethal damage to it.
fn prevents_combat_damage_to_this(_: &dyn Query, asker: &Asker, e: &DamageEvent) -> bool {
    e.combat && matches!(e.target, Target::Object(o) if o == asker.source)
}

/// "This spell costs {1} less to cast for each permanent card in your graveyard" (CR 601.2f, 118.7a, 110.4a): counted as the
/// cost is determined, the cards as they are in the graveyard; only the cost of casting it, its mana value unchanged (CR 118.7).
fn costs_one_less_for_each_permanent_card(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(targets::permanent_cards_in_graveyard(q, player) as u32)
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[costs_one_less_for_each_permanent_card],
    preventions: &[PreventionDef {
        hint: "prevents all combat damage that would be dealt to this creature",
        applies: prevents_combat_damage_to_this,
        amount: PreventionAmount::All,
    }],
    ..oracle::DIAMOND_WEAPON
};
