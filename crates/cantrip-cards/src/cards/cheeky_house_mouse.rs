use cantrip_api::*;
use cantrip_mechanics::{combat, pump, targets};
use cantrip_oracle as oracle;

/// "Can't be blocked by creatures with power 3 or greater": asked of each possible block while the effect lasts, the blocker's
/// power as blockers are declared (CR 509.1b); a block already made stands if powers change afterwards (the rulings).
const NO_BIG_BLOCKERS: LastingBlockRule =
    LastingBlockRule { text: "Can't be blocked by creatures with power 3 or greater", allows: combat::power_three_or_greater_cant_block };

/// "Target creature you control gets +1/+1 until end of turn. It can't be blocked by creatures with power 3 or greater this
/// turn." The second sentence is a resolved effect on the creature, not an ability it gains (CR 113.1a, 611.2c). With its one
/// target illegal the spell does not resolve (CR 608.2b).
fn squeak_by(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 1);
        g.add_effect(item.object, creature, EffectKind::BlockRestriction { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
    });
    Ok(())
}

/// The Adventure (CR 715): Squeak By.
pub static SQUEAK_BY: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], squeak_by),
    lasting_block_rules: &[NO_BIG_BLOCKERS],
    ..oracle::SQUEAK_BY
};

pub static CARD: CardDef = CardDef { adventure: Some(&SQUEAK_BY), ..oracle::CHEEKY_HOUSE_MOUSE };

pub static EXTRA: &[&CardDef] = &[&SQUEAK_BY];
