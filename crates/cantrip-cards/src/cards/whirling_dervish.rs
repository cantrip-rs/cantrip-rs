use cantrip_api::*;
use cantrip_mechanics::{effects, protection, triggers};
use cantrip_oracle as oracle;

/// Whether this creature dealt damage to an opponent of its current controller this turn (CR 603.4).
fn dealt_damage_to_opponent(q: &dyn Query, asker: &Asker) -> bool {
    // A player who has since left still counts (the card's ruling), so do not use `opponents()`.
    q.history().damage_dealt().any(|d| {
        d.source == asker.source && d.amount > 0 && matches!(d.target, Target::Player(player) if player != asker.controller)
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::EACH_END_STEP,
            effects::plus_one_counter_on_this,
            "At the beginning of each end step, if this creature dealt damage to an opponent this turn, put a +1/+1 counter on it.",
        )
        .condition(dealt_damage_to_opponent),
    ],
    statics: &[protection::from::<{ Color::Black as u8 }>("Protection from black")],
    ..oracle::WHIRLING_DERVISH
};
