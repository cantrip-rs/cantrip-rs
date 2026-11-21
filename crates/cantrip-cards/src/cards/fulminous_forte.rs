use cantrip_api::*;
use cantrip_mechanics::{effects, targets, damage_target};
use cantrip_oracle as oracle;
/// "Fulminous Forte deals 1 damage to each creature and planeswalker your opponents control": one simultaneous damage event to
/// each (CR 120.3, 608.2c).
fn sweep(g: &mut dyn Game, i: &StackItem) -> Result<(), Suspend> {
 let you = i.controller;
 effects::damage_each_creature_and_planeswalker_where(g, i, 1, |q, id| q.state().opponents(you).any(|o| o == q.controller(id)));
 Ok(())
}
static MODAL: ModalDef = modes(&[
 mode(sweep, "Fulminous Forte deals 1 damage to each creature and planeswalker your opponents control."),
 mode(|g,i| { damage_target(g,i,0,5); Ok(()) }, "Fulminous Forte deals 5 damage to target creature or planeswalker.").targets(&[targets::CREATURE_OR_PLANESWALKER]),
]);
pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::FULMINOUS_FORTE };
