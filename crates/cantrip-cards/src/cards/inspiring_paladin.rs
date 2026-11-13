use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "During your turn, creatures you control with +1/+1 counters on them have first strike" (CR 604.1, 613.1f): the creatures its
/// controller controls that have a +1/+1 counter, only while that player is the active player (CR 102.1); asked again whenever
/// characteristics are computed, so it follows the counters, the turn and the Paladin itself.
fn your_creatures_with_counters_during_your_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    q.state().turn.active == q.controller(source) && statics::your_creatures_with_plus_one_counters(q, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, statics::itself_during_your_turn, statics::have_first_strike, "During your turn, this creature has first strike."),
        static_ability(
            Layer::Ability,
            your_creatures_with_counters_during_your_turn,
            statics::have_first_strike,
            "During your turn, creatures you control with +1/+1 counters on them have first strike.",
        ),
    ],
    ..oracle::INSPIRING_PALADIN
};
