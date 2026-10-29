use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control deals damage equal to its power to target creature an opponent controls." (CR 120.3, 609.7,
/// 608.2b): one-sided, the creature being the source of the damage, so its abilities (lifelink, deathtouch) and colors count
/// (`fight::bite`), and its power the one it has as the spell resolves. If either target is illegal then, no damage is dealt.
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::targets_bite(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], deal_damage),
    ..oracle::TENDERIZE
};
