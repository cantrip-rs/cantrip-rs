use cantrip_api::*;
use cantrip_mechanics::{copy, effects, triggers};
use cantrip_oracle as oracle;

/// "You may copy it. You may choose new targets for the copy" (CR 707.10, 707.10c): the copy is of the Adventure spell that was
/// cast, made if its controller says so; it is not cast, and as it resolves it is exiled with nothing to cast later (CR 715.3c, 715.3d,
/// 707.10a, 111.1).
fn may_copy_the_adventure_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        copy::copy_the_cast_spell(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::ADVENTURE_SPELL_YOU_CAST,
        may_copy_the_adventure_spell,
        "Whenever you cast an Adventure spell, you may copy it. You may choose new targets for the copy.",
    )],
    ..oracle::CHANCELLOR_OF_TALES
};
