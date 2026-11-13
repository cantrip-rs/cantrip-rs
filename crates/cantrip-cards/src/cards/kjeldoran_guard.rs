use cantrip_api::*;
use cantrip_mechanics::{combat, delayed, effects, history, lands, targets, triggers};
use cantrip_oracle as oracle;

/// "{T}: Target creature gets +1/+1 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat and only if defending player controls no snow lands." (CR 602.5).
fn target_creature_gets_1_1_if(q: &dyn Query, asker: &Asker) -> bool {
    if !combat::during_combat(q) {
        return false;
    }
    // Only an assignment in this combat makes this creature's defender relevant (CR 802.2a).
    let assigned_this_combat = history::this_combat(q).iter()
        .any(|r| matches!(r.event,
            Event::Attacks { attacker, .. } | Event::AttackAssigned { attacker, .. } if attacker == asker.source));
    if let Some(defending) = q.defending_player(asker.source).filter(|_| assigned_this_combat) {
        return !lands::controls_snow_land(q, defending);
    }
    q.state().opponents(q.state().turn.active).any(|defending| !lands::controls_snow_land(q, defending))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                delayed::target_creature_gets_then_sacrifice_this_when_it_leaves::<1, 1>,
                "{T}: Target creature gets +1/+1 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat and only if defending player controls no snow lands.",
            )
            .targets(&[targets::CREATURE])
            .condition(target_creature_gets_1_1_if),
    ],
    delayed: &[
        triggered(
            triggers::THIS_LEAVES,
            effects::sacrifice_self,
            "When that creature leaves the battlefield this turn, sacrifice this creature.",
        ),
    ],
    ..oracle::KJELDORAN_GUARD
};
