use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::attach::equip_targeting_this_costs_less;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        effects::first_target_gains_flying,
        "Whenever this creature attacks, another target attacking creature gains flying until end of turn.",
    )
    .targets(&[targets::ANOTHER_ATTACKING_CREATURE])],
    // "Equip abilities you activate that target this creature cost {2} less to activate." (CR 601.2f, 602.2b)
    cost_changes: &[equip_targeting_this_costs_less::<2>],
    ..oracle::HELITROOPER
};
