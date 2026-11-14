use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Green creatures can't attack unless their controller sacrifices a land of their choice for each green creature they control that's attacking. (This cost is paid as attackers are declared.)" (CR 604.1).
fn green_creatures_cant_attack_unless_rule(
    q: &dyn Query,
    asker: &Asker,
    attacker: ObjectId,
    _: Target,
) -> bool {
    q.chars(attacker).colors.has(q.words(asker.source).color(Color::Green))
        && q.chars(attacker).is(CardType::Creature)
}

/// "Green creatures can't attack unless their controller sacrifices a land of their choice for each green creature they control that's attacking. (This cost is paid as attackers are declared.)" (CR 118.1).
fn green_creatures_cant_attack_unless_rule_cost_0(
    q: &dyn Query,
    _: &Asker,
    id: ObjectId,
) -> bool {
    q.chars(id).is(CardType::Land)
}

pub static CARD: CardDef = CardDef {
    attack_cost_rules: &[
        AttackPayment {
            applies: green_creatures_cant_attack_unless_rule,
            cost: sacrifice(1, green_creatures_cant_attack_unless_rule_cost_0),
            grouped: true,
        },
    ],
    ..oracle::FLOODED_WOODLANDS
};
