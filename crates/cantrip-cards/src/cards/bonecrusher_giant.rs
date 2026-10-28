use cantrip_api::*;
use cantrip_mechanics::{targets, damage_target};
use cantrip_oracle as oracle;

fn stomp(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // The first sentence is a rule of the game until end of turn (CR 615.12), made even if the target is gone.
    g.damage_cant_be_prevented();
    damage_target(g, item, 0, 2);
    Ok(())
}

/// The Adventure (CR 715): Damage can't be prevented this turn. Stomp deals 2 damage to any target.
pub static STOMP: CardDef = CardDef {
    spell: spell(&[targets::ANY], stomp),
    ..oracle::STOMP
};

/// "Whenever this creature becomes the target of a spell": a spell, not an ability, from any player.
fn targeted_by_spell(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::BecameTarget { target: Target::Object(t), controller, ability: false, .. } if *t == source => {
            Fired::about(Target::Player(*controller))
        }
        _ => None,
    }
}

fn two_to_its_controller(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.damage(item.object, item.subject, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::BECAME_TARGET, targeted_by_spell), two_to_its_controller, "Whenever this creature becomes the target of a spell, this creature deals 2 damage to that spell's controller.")],
    adventure: Some(&STOMP),
    ..oracle::BONECRUSHER_GIANT
};

pub static EXTRA: &[&CardDef] = &[&STOMP];
