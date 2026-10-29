use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_minus_one_counters::<1>),
    abilities: &[ability(
        cost("{1}{W}").and(REMOVE_A_COUNTER),
        |g, item| effects::draw(g, item, 1),
        "{1}{W}, Remove a counter from this creature: Draw a card. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    ..oracle::MOONLIT_LAMENTER
};
