use cantrip_api::*;
use cantrip_mechanics::{attach, targets, triggers};
use cantrip_oracle as oracle;

/// "When this Aura enters, tap enchanted creature." (CR 303.4m, 701.26a).
fn tap_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    attach::tap_attached(g, item);
    Ok(())
}

/// The enchanted creature does not untap if it attacked during its controller's last turn (CR 502.3).
fn attacked_during_controllers_last_turn(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    if !attach::attached_to_it(q, asker, id) {
        return false;
    }
    let controller = q.controller(id);
    let last_turn = q.history().all().iter().rev().find_map(|record| {
        match record.event {
            Event::TurnBegan { player, .. } if player == controller && record.turn < q.state().turn.number => Some(record.turn),
            _ => None,
        }
    });
    q.history().all().iter().any(|record| {
        Some(record.turn) == last_turn && matches!(record.event, Event::Attacks { attacker, .. } if attacker == id)
    })
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(triggers::THIS_ENTERS, tap_enchanted, "When this Aura enters, tap enchanted creature.")],
    untap_bans: &[attacked_during_controllers_last_turn],
    ..oracle::TANGLE_KELP
};
