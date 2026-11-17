use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        statics::itself_while_you_drew_two_or_more,
        statics::have_lifelink_and_menace,
        "As long as you've drawn two or more cards this turn, this creature has lifelink and menace.",
    )],
    ..oracle::FOGGY_SWAMP_HUNTERS
};
