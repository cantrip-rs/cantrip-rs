use cantrip_api::*;
use cantrip_mechanics::crew::becomes_a_creature;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Two untapped tokens you control": tapped to pay the cost (CR 118.3), so a token that came under its controller's control this
/// turn counts (CR 302.6 limits only {T}).
fn untapped_token_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    targets::token_you_control(q, asker, id) && !q.obj(id).tapped
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{3}, {T}"), effects::plus_one_counters_on_target::<1>, "{3}, {T}: Put a +1/+1 counter on target creature.").targets(&[targets::CREATURE]),
        ability(
            Cost::tap_untapped(2, untapped_token_you_control),
            becomes_a_creature,
            "Tap two untapped tokens you control: This Vehicle becomes an artifact creature until end of turn.",
        ),
    ],
    ..oracle::TANGLE_TUMBLER
};
