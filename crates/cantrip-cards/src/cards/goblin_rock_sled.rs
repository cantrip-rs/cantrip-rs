use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "This creature doesn't untap during your untap step if it attacked during your last turn. This creature can't attack unless defending player controls a Mountain." (CR 604.1).
fn creature_doesnt_untap_during_untap_rule(
    q: &dyn Query,
    asker: &Asker,
    id: ObjectId,
) -> bool {
    (Some(asker.source)
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
        })) && id == asker.source
}

/// The defending player must control a Mountain, including when attacking a battle (CR 508.5, 612.2).
fn attack_rule(q: &dyn Query, asker: &Asker, attacker: ObjectId, defender: Target) -> bool {
    attacker != asker.source || defending_player_of(q.state(), defender)
        .is_none_or(|player| lands::controls_land_type(q, player, q.words(asker.source).subtype(Subtype::Mountain)))
}

pub static CARD: CardDef = CardDef {
    attack_rules: &[attack_rule],
    untap_bans: &[creature_doesnt_untap_during_untap_rule],
    ..oracle::GOBLIN_ROCK_SLED
};
