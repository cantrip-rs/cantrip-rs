use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::{have_deathtouch, plus_pt};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "When enchanted creature dies" (CR 603.6c, 603.10a): a leaves-the-battlefield ability of the Aura, looked back on the
/// board before the event, so it still sees what it was attached to. Fires on the card as it is in the graveyard (the new
/// object, CR 400.7).
fn enchanted_creature_dies(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let (_, new) = cantrip_mechanics::triggers::enchanted_creature_died(q, source, event)?;
    Fired::about(Target::Object(new))
}

/// "Return that card to the battlefield tapped under its owner's control" (CR 110.2a): the card as it is in the graveyard.
fn return_host(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(card) = item.subject else { return Ok(()) };
    let owner = g.obj(card).owner;
    g.put_onto_battlefield(card, owner, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, enchanted_creature_dies), return_host, "When enchanted creature dies, return that card to the battlefield tapped under its owner's control.")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 0>, "Enchanted creature gets +1/+0."),
        static_ability(Layer::Ability, attached_object, have_deathtouch, "Enchanted creature has deathtouch."),
    ],
    ..oracle::FEROCITY_OF_THE_HUNT
};
