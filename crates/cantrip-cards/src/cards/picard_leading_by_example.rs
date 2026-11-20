use cantrip_api::*;
use cantrip_mechanics::effects::target_creature_gets_plus_x_x_for_this_power;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(TAP),
        target_creature_gets_plus_x_x_for_this_power,
        "{1}, {T}: Another target creature gets +X/+X until end of turn, where X is Picard's power. Activate only as a sorcery.",
    )
    .targets(&[targets::OTHER_CREATURE])
    .sorcery_speed()],
    ..oracle::PICARD_LEADING_BY_EXAMPLE
};
