use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::expend::EXPEND_4;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            EXPEND_4,
            effects::plus_one_counter_on_this,
            "Whenever you expend 4, put a +1/+1 counter on this creature. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
        ),
        mana_ability(TAP, add_one_of(&[Color::Red, Color::Green]), "{T}: Add {R} or {G}."),
    ],
    ..oracle::WANDERTALE_MENTOR
};
