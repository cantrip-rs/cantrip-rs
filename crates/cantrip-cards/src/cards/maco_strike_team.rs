use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Whenever a creature you control becomes the target of a spell or ability an opponent controls" (CR 603.2, 115.10): each
/// creature you control that becomes a target is its own event. Only a creature on the battlefield is "a creature you
/// control", and only a spell or ability one of this permanent's controller's opponents controls counts.
fn your_creature_targeted_by_an_opponent(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::BecameTarget { target: Target::Object(t), controller, .. }
            if q.state().opponents(me.controller).any(|p| p == *controller)
                && q.is_on_battlefield(*t)
                && q.controller(*t) == me.controller
                && q.chars(*t).is(CardType::Creature) =>
        {
            Fired::about(Target::Object(*t))
        }
        _ => None,
    }
}

/// "Create a tapped 1/1 red Officer creature token" (CR 111.1): one, tapped, under this creature's controller.
fn create_tapped_officer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_by_rule(g, item.controller, &tokens::OFFICER, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::BECAME_TARGET, your_creature_targeted_by_an_opponent),
        create_tapped_officer,
        "Whenever a creature you control becomes the target of a spell or ability an opponent controls, create a tapped 1/1 red Officer creature token.",
    )],
    ..oracle::MACO_STRIKE_TEAM
};
