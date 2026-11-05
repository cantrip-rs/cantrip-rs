use cantrip_api::*;
use cantrip_mechanics::{empower, targets};
use cantrip_oracle as oracle;

/// "Violent Echoes deals 6 damage to target creature or planeswalker. If excess damage was dealt to that permanent
/// this way, empower Jace X, where X is that excess damage." The damage procedure determines actual excess after
/// replacement and prevention (CR 120.10). Empower Jace uses the shared keyword action (CR 701.71a).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = item.target(0);
    if let Some(result) = g.damage_result(item.object, target, 6)
        && result.target == target && result.excess > 0
    {
        empower::jace(g, item.controller, item.object, result.excess.min(i64::from(i16::MAX as u32)) as i16)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_OR_PLANESWALKER], effect),
    ..oracle::VIOLENT_ECHOES
};

pub static EXTRA: &[&CardDef] = &[&empower::JACE];
