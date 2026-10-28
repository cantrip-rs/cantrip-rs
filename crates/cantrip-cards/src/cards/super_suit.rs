use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this Equipment enters, attach it to target creature you control. Untap that creature." The creature is the
/// target, so it untaps even if the Equipment has left the battlefield by now and is not attached (CR 701.3b); a target no
/// longer legal means the trigger does nothing (CR 608.2b).
fn attach_and_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.attach(item.object, creature);
    g.end_sentence();
    g.untap(creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, attach_and_untap, "When this Equipment enters, attach it to target creature you control. Untap that creature.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 2>, "Equipped creature gets +1/+2.")],
    ..oracle::SUPER_SUIT
};
