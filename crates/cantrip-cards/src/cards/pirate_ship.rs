use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This creature can't attack unless defending player controls an Island. {T}: This creature deals 1 damage to any target. When you control no Islands, sacrifice this creature." (CR 604.1).
fn creature_deals_1_damage_any_rule(
    q: &dyn Query,
    asker: &Asker,
    attacker: ObjectId,
    defender: Target,
) -> bool {
    let asker = &Asker::new(asker.source, q.controller(asker.source));
    !(attacker == asker.source
        && defending_player_of(q.state(), defender)
            .is_some_and(|defending| {
                q
                    .state()
                    .apnap()
                    .filter(|&bound_0: &PlayerId| {
                        !(q
                            .state()
                            .battlefield()
                            .iter()
                            .copied()
                            .filter(|&id| {
                                q.controller(id)
                                    == Some(bound_0).unwrap_or(PlayerId(u8::MAX))
                                    && q
                                        .chars(id)
                                        .has_subtype(q.words(asker.source).subtype(Subtype::Island))
                            })
                            .count() >= 1)
                    })
                    .collect::<Vec<PlayerId>>()
                    .contains(&defending)
            }))
}

/// "This creature can't attack unless defending player controls an Island. {T}: This creature deals 1 damage to any target. When you control no Islands, sacrifice this creature." (CR 608.2).
fn creature_deals_1_damage_any_state(q: &dyn Query, asker: &Asker) -> bool {
    (q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Island))
                && asker.controller == q.controller(id)
        })
        .collect::<Vec<ObjectId>>()
        .len() as i64) == 0
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                effects::damage_first_target::<1>,
                "{T}: This creature deals 1 damage to any target.",
            )
            .targets(&[targets::ANY]),
        state_triggered(
            creature_deals_1_damage_any_state,
            effects::sacrifice_self,
            "When you control no Islands, sacrifice this creature.",
        ),
    ],
    attack_rules: &[creature_deals_1_damage_any_rule],
    ..oracle::PIRATE_SHIP
};
