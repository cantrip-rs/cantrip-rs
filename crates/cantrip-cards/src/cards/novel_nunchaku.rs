use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{have_trample, plus_pt};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "When this Equipment enters, attach it to target creature you control. When you do, ..." (CR 701.3a, 603.12): the reflexive
/// ability triggers only when the Equipment really was attached, which it is not if the creature is no longer a legal target
/// or it has left the battlefield.
fn attach_then_fight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.attach(item.object, creature);
    if g.obj(item.object).attached_to == Some(creature) {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            attach_then_fight,
            "When this Equipment enters, attach it to target creature you control. When you do, equipped creature fights up to one target creature an opponent controls.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{3}"), "Equip {3}"),
    ],
    delayed: &[delayed_trigger(
        fight::attached_creature_fights_first_target,
        "When you do, equipped creature fights up to one target creature an opponent controls.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS.optional()])],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, have_trample, "Equipped creature has trample."),
    ],
    ..oracle::NOVEL_NUNCHAKU
};
