use cantrip_api::*;
use cantrip_mechanics::{damage_divided, mana, targets};
use cantrip_oracle as oracle;

/// X damage divided among up to two targets; if X is 6 or more, twice X damage instead (CR 601.2d): the total is fixed
/// as the spell is cast, and each target takes its own share, one that has since become illegal being left out with it.
fn total(x: u16) -> u16 {
    // Doubled in u32 so a large X cannot overflow; damage is a u16 (RX-5), so the product is held at its largest value.
    if x >= 6 { u16::try_from(u32::from(x) * 2).unwrap_or(u16::MAX) } else { x }
}

fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_divided(g, item);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::R), "{T}: Add {R}.")],
    enters: Some(mana::enters_tapped_unless_pay_three_life),
    ..oracle::SHATTERSKULL_THE_HAMMER_PASS
};

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER.between(0, 2)], effect),
    divide: Some(Divide { total: TargetSpec::COUNTED, of_x: Some(total) }),
    back: Some(&BACK),
    ..oracle::SHATTERSKULL_SMASHING
};
