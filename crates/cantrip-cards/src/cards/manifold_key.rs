use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}, {T}"), effects::untap_targets, "{1}, {T}: Untap another target artifact.")
            .targets(&[targets::OTHER_ARTIFACT]),
        ability(cost("{3}, {T}"), effects::target_creature_cant_be_blocked_this_turn, "{3}, {T}: Target creature can't be blocked this turn.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::MANIFOLD_KEY
};
