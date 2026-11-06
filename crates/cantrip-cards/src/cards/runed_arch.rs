use cantrip_api::*;
use cantrip_mechanics::{mana, targets};
use cantrip_oracle as oracle;

/// "X target creatures with power 2 or less can't be blocked this turn" (CR 608.2b, 611.2a).
fn creatures_cant_be_blocked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    instruction(g, |g| {
        for &creature in &creatures {
            g.add_effect(item.object, creature, EffectKind::CantBeBlocked, Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{X}, {T}").and(SACRIFICE_THIS),
        creatures_cant_be_blocked,
        "{X}, {T}, Sacrifice this artifact: X target creatures with power 2 or less can't be blocked this turn.",
    ).targets(&[targets::CREATURE_WITH_POWER_2_OR_LESS.counted()])],
    enters: Some(mana::enters_tapped),
    ..oracle::RUNED_ARCH
};
