use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{R}, Sacrifice this creature: It deals 2 damage to target creature it's blocking." (CR 115.1).
fn deals_2_damage_target_creature_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(t, Target::Object(id)
        if targets::permanent(q, id)
            && q.chars(id).is(CardType::Creature)
            && q.creatures_blocked_by(asker.source).contains(&id))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            SACRIFICE_THIS,
            add_n(Mana::R, 2),
            "Sacrifice this creature: Add {R}{R}.",
        ),
        ability(
                cost("{R}").and(SACRIFICE_THIS),
                effects::damage_first_target::<2>,
                "{R}, Sacrifice this creature: It deals 2 damage to target creature it's blocking.",
            )
            .targets(&[target(deals_2_damage_target_creature_target_0)]),
    ],
    ..oracle::TINDER_WALL
};
