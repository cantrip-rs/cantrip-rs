use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{1}{W}: Target Soldier creature gets +1/+0 until end of turn." (CR 115.1).
fn target_soldier_creature_gets_1_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    (targets::CREATURE.filter)(q, asker, t)
        && targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Soldier])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{W}"),
                effects::target_creature_gets::<1, 0>,
                "{1}{W}: Target Soldier creature gets +1/+0 until end of turn.",
            )
            .targets(&[target(target_soldier_creature_gets_1_target_0)]),
    ],
    ..oracle::ICATIAN_LIEUTENANT
};
