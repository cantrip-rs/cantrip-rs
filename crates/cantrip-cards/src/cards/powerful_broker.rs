use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target permanent or player" (CR 115.1): any permanent on the battlefield, or any player.
fn permanent_or_player(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::PERMANENT.filter)(q, asker, t) || (targets::PLAYER.filter)(q, asker, t)
}

/// "For each kind of counter on target permanent or player, give that permanent or player another counter of that kind." (CR 122.1,
/// 122.6, 608.2b): one more of every kind it has as the ability resolves, each kind counted once however many counters of it there
/// are; a player's counters are poison, energy and experience (CR 122.1). The permanent's are put on at once as one placement; the
/// player's by this ability's controller, who gives them (CR 122.6a). A target that is gone, or has no counters, gets none.
fn give_another_of_each_kind(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(object) = targets::object(g, item, 0) {
        let have = g.obj(object).counter_array();
        let puts: Vec<(ObjectId, CounterKind, i16)> =
            CounterKind::ALL.into_iter().filter(|&kind| have[kind as usize] > 0).map(|kind| (object, kind, 1)).collect();
        g.add_counters_together(&puts);
        return Ok(());
    }
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let (poison, energy, experience) = {
        let counters = g.state().player(player);
        (counters.poison() > 0, counters.energy() > 0, counters.experience() > 0)
    };
    if poison {
        g.give_poison(player, 1);
    }
    if energy {
        g.change_energy(player, 1);
    }
    if experience {
        g.give_experience(player, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, give_another_of_each_kind, "{T}: For each kind of counter on target permanent or player, give that permanent or player another counter of that kind. Activate only as a sorcery.")
        .targets(&[target(permanent_or_player)])
        .sorcery_speed()],
    ..oracle::POWERFUL_BROKER
};
