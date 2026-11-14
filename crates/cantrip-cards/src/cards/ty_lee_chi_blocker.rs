use cantrip_api::*;
use cantrip_mechanics::triggers::{PROWESS, THIS_ENTERS};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Tap up to one target creature. It doesn't untap during its controller's untap step for as long as you control Ty
/// Lee." (CR 502.3, 611.2b): it ends the first time Ty Lee is gone or another player controls her, and never comes back
/// (the rulings); if she has already left or changed hands as this resolves, the creature is still tapped but untaps
/// normally.
fn block(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.end_sentence();
    let until = Duration::WhileSourceControlled;
    g.add_effect(item.object, creature, EffectKind::DoesntUntap, until);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        PROWESS,
        triggered(
            THIS_ENTERS,
            block,
            "When Ty Lee enters, tap up to one target creature. It doesn't untap during its controller's untap step for as long as you control Ty Lee.",
        )
        .targets(&[targets::CREATURE.optional()]),
    ],
    ..oracle::TY_LEE_CHI_BLOCKER
};
