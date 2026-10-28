use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{sacrifice_chosen, statics, targets};
use cantrip_oracle as oracle;

/// "Target opponent sacrifices a creature of their choice" (CR 701.21a): they choose which, and nothing happens if they have
/// none or the target is no longer legal (CR 608.2b).
fn opponent_sacrifices_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    sacrifice_chosen(g, opponent, item.object, 1, |chars| chars.is(CardType::Creature))?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, opponent_sacrifices_a_creature, "When this Equipment enters, target opponent sacrifices a creature of their choice.")
            .targets(&[targets::OPPONENT]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, statics::plus_pt::<2, 1>, "Equipped creature gets +2/+1.")],
    ..oracle::CRUDE_BENT_BLADE
};
