use cantrip_api::*;
use cantrip_mechanics::history::gained_life_this_turn;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "When you cast this spell, copy it if you gained life this turn. You may choose new targets for the copy." The trigger always
/// happens; whether life was gained is judged as it resolves (CR 603.4 does not apply, the "if" is an instruction, CR 608.2c). The
/// copy is not cast (CR 707.10).
fn copy_if_gained_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if gained_life_this_turn(g, item.controller) {
        g.copy_spell(item.object, item.controller, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets::<2, 4>),
    abilities: &[triggered(
        triggers::CAST_THIS_SPELL,
        copy_if_gained_life,
        "Infusion \u{2014} When you cast this spell, copy it if you gained life this turn. You may choose new targets for the copy.",
    )],
    ..oracle::LUMARETS_FAVOR
};
