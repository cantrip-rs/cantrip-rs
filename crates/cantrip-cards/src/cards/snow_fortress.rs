use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// A creature without flying that is attacking you (CR 115.1, 506.2).
fn ground_attacker_at_you(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if !q.chars(id).has(Keyword::Flying)
            && q.state().attackers.iter().any(|a| a.id == id && a.target == Target::Player(asker.controller)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), effects::this_gets::<1, 0>, "{1}: This creature gets +1/+0 until end of turn."),
        ability(cost("{1}"), effects::this_gets::<0, 1>, "{1}: This creature gets +0/+1 until end of turn."),
        ability(cost("{3}"), effects::damage_first_target::<1>, "{3}: This creature deals 1 damage to target creature without flying that's attacking you.")
            .targets(&[target(ground_attacker_at_you)]),
    ],
    ..oracle::SNOW_FORTRESS
};
