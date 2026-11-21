use cantrip_api::*;
use cantrip_mechanics::{delayed, targets, triggers};
use cantrip_oracle as oracle;

/// "Choose target creature you control and target Equipment. Gain control of that Equipment until end of turn. Attach it to
/// the chosen creature. When you lose control of that Equipment this turn, if it's attached to a creature you control,
/// unattach it." The control is gained whether or not the creature is still a legal target (CR 608.2b; its ruling: "you'll
/// still gain control of the target Equipment until end of turn, but you won't attach it"). The delayed ability (CR 603.7b)
/// watches the Equipment.
fn stolen_uniform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(equipment) = targets::object(g, item, 1) else { return Ok(()) };
    g.gain_control(item.object, equipment, item.controller, Duration::EndOfTurn);
    g.end_sentence();
    if let Some(creature) = targets::object(g, item, 0) {
        g.attach(equipment, creature);
    }
    delayed::this_turn(g, item, equipment);
    Ok(())
}

/// "If it's attached to a creature you control" (CR 603.4): whether `equipment` is attached to a creature `you` control.
fn attached_to_your_creature(q: &dyn Query, equipment: ObjectId, you: PlayerId) -> bool {
    q.state()
        .obj(equipment)
        .attached_to
        .is_some_and(|host| q.is_on_battlefield(host) && q.controller(host) == you && q.chars(host).is(CardType::Creature))
}

/// "When you lose control of that Equipment this turn, if it's attached to a creature you control" (CR 603.4): the delayed
/// ability watches the Equipment, whose events and attachment its trigger is asked of (`me.source`); its own source is this
/// spell (CR 603.7d), so the intervening "if" is asked here as it triggers, and again as it resolves.
fn you_lose_control_of_it_while_attached(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::you_lose_control_of_this(q, me, event).filter(|_| attached_to_your_creature(q, me.source, me.controller))
}

/// "Unattach it" (CR 701.3d), if it is still attached to a creature you control (CR 603.4): "it" is the Equipment watched,
/// the ability's subject.
fn unattach_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(equipment) = item.subject.object() else { return Ok(()) };
    if attached_to_your_creature(g, equipment, item.controller) {
        g.unattach(equipment);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::EQUIPMENT], stolen_uniform),
    delayed: &[triggered(
        when_asked(EventMask(EventMask::CONTROL_CHANGED.0 | EventMask::ZONE_CHANGE.0), you_lose_control_of_it_while_attached),
        unattach_it,
        "When you lose control of that Equipment this turn, if it's attached to a creature you control, unattach it.",
    )],
    ..oracle::STOLEN_UNIFORM
};
