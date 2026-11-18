use cantrip_api::*;
use cantrip_mechanics::{effects::plus_one_counter_on_this, triggers};
use cantrip_oracle as oracle;

/// "If a face-down creature entered the battlefield under your control this turn" (CR 603.4, 708.2, 708.3): a permanent that
/// entered face down is a creature (CR 708.2a), and this turn's history keeps it as it entered, whatever has become of it since
/// (turned face up, died, or gone to another zone or controller).
fn face_down_creature_entered_under_your_control_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    let you = q.controller(asker.source);
    q.history().any(EventQuery::kind(EventMask::ENTERED_FACE_DOWN).this_turn().player(you))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_END_STEP,
            plus_one_counter_on_this,
            "At the beginning of your end step, if a face-down creature entered the battlefield under your control this turn, put a +1/+1 counter on this creature.",
        )
        .condition(face_down_creature_entered_under_your_control_this_turn),
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
    ],
    ..oracle::TUNNEL_TIPSTER
};
