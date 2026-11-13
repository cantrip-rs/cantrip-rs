use cantrip_api::*;
use cantrip_mechanics::{attach, history, targets, triggers};
use cantrip_oracle as oracle;

/// The enchanted creature does not untap if it attacked during its controller's last turn (CR 502.3).
fn attacked_during_controllers_last_turn(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    attach::attached_to_it(q, asker, id) && history::attacked_during_last_turn_of(q, q.controller(id), id)
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(triggers::THIS_ENTERS, attach::tap_enchanted, "When this Aura enters, tap enchanted creature.")],
    untap_bans: &[attacked_during_controllers_last_turn],
    ..oracle::TANGLE_KELP
};
