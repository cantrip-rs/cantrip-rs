use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature you control gets +1/+1 and target creature an opponent controls gets -1/-1." (CR 608.2b,
/// 611.2a): each target is judged on its own as this resolves, so one that has become illegal does not stop the other from taking
/// its modifier; only if both are illegal does the spell not resolve, and that is the core's.
fn grow_and_shrink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.one_instruction(&mut |g| {
        if let Some(yours) = targets::object(g, item, 0) {
            pump(g, item.object, yours, 1, 1);
        }
        if let Some(theirs) = targets::object(g, item, 1) {
            pump(g, item.object, theirs, -1, -1);
        }
        Ok(())
    })
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], grow_and_shrink),
    ..oracle::SKULDUGGERY
};
