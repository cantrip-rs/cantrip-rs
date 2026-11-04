use cantrip_api::*;
use cantrip_mechanics::{mana, targets};
use cantrip_oracle as oracle;

/// Put a +1/+1 counter on target creature. It gains indestructible until end of turn.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::plus_one_counter_on_target_and_gain_until_end_of_turn(g, item, Keyword::Indestructible)
}

pub static BACK: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_one_of(&[Color::Black, Color::Green]), "{T}: Add {B} or {G}.")],
    enters: Some(mana::enters_tapped),
    ..oracle::OLD_GROWTH_GROVE
};

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    back: Some(&BACK),
    ..oracle::REVITALIZING_REPAST
};
