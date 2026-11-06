use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// A creature must currently be blocking this creature in the declare blockers step (CR 602.5, 509.1).
fn creature_gets_2_0_activate_if(q: &dyn Query, asker: &Asker) -> bool {
    q.state().turn.step() == Step::DeclareBlockers
        && q.state().blocks.iter().any(|b| {
            b.attacker == asker.source && q.chars(b.blocker).is(CardType::Creature)
        })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{R}"),
                effects::this_gets::<2, 0>,
                "{R}: This creature gets +2/+0 until end of turn. Activate only during the declare blockers step, only if at least one creature is blocking this creature, and only once each turn.",
            )
            .once_each_turn()
            .condition(creature_gets_2_0_activate_if),
    ],
    ..oracle::GRIZZLED_WOLVERINE
};
