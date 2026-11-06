use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't block white creatures with power 2 or greater. {R}: This creature gains first strike until end of turn." (CR 604.1).
fn creature_gains_first_strike_rule(
    q: &dyn Query,
    source: ObjectId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(blocker == asker.source
        && (q.chars(attacker).colors.has(q.words(asker.source).color(Color::White))
            && q.chars(attacker).is(CardType::Creature)
            && q.chars(attacker).power.is_some_and(|v| (i64::from(v)) >= 2)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{R}"),
            cantrip_mechanics::effects::this_gains_first_strike,
            "{R}: This creature gains first strike until end of turn.",
        ),
    ],
    block_rules: &[creature_gains_first_strike_rule],
    ..oracle::ORCISH_VETERAN
};
