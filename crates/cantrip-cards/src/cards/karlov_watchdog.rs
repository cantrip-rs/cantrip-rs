use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Permanents your opponents control can't be turned face up during your turn" (CR 101.2, 708.7): a rule of the Watchdog's, true of a
/// permanent an opponent of its controller controls while it is its controller's turn, judged as the permanent would be turned up.
/// It is no ability of the permanent, and stops every way of turning it up, the disguise special action included.
fn opponents_permanents_during_your_turn(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.state().turn.active == asker.controller && q.controller(id) != asker.controller
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKERS_DECLARED, triggers::you_attack_with_at_least::<3>),
        effects::creatures_you_control_get::<1, 1>,
        "Whenever you attack with three or more creatures, creatures you control get +1/+1 until end of turn.",
    )],
    face_up_bans: &[opponents_permanents_during_your_turn],
    ..oracle::KARLOV_WATCHDOG
};
