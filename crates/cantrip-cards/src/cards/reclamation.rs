use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// "Black creatures can't attack unless their controller sacrifices a land of their choice for each black creature they control that's attacking. (This cost is paid as attackers are declared.)" (CR 604.1).
fn black_creatures_cant_attack_unless_rule(
    q: &dyn Query,
    asker: &Asker,
    attacker: ObjectId,
    _: Target,
) -> bool {
    q.chars(attacker).colors.has(q.words(asker.source).color(Color::Black))
        && q.chars(attacker).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    attack_cost_rules: &[
        AttackPayment {
            applies: black_creatures_cant_attack_unless_rule,
            cost: sacrifice(1, filters::land),
        },
    ],
    ..oracle::RECLAMATION
};
