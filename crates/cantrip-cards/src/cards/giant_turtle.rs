use cantrip_api::*;
use cantrip_oracle as oracle;

/// "This creature can't attack if it attacked during your last turn." (CR 604.1).
fn creature_cant_attack_if_attacked_rule(
    q: &dyn Query,
    asker: &Asker,
    attacker: ObjectId,
    _: Target,
) -> bool {
    let asker = &Asker::new(asker.source, q.controller(asker.source));
    !((Some(asker.source)
        .is_some_and(|candidate: ObjectId| {
            {
                let turn = q
                    .history()
                    .all()
                    .iter()
                    .rev()
                    .find(|r| {
                        r.active == asker.controller && r.turn < q.state().turn.number
                    })
                    .map(|r| r.turn);
                q.history().all().iter().filter(move |r| Some(r.turn) == turn)
            }
                .any(|r| {
                    matches!(
                        r.event, Event::Attacks { attacker, .. } if attacker == candidate
                    )
                })
        })) && attacker == asker.source)
}

pub static CARD: CardDef = CardDef {
    attack_rules: &[creature_cant_attack_if_attacked_rule],
    ..oracle::GIANT_TURTLE
};
