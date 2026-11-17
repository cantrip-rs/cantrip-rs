use cantrip_api::*;
use cantrip_mechanics::attach::source_remains_attached_to_it;
use cantrip_oracle as oracle;

/// "Whenever an Aura you control becomes attached to a nonland permanent an opponent controls with mana value less than or equal
/// to that Aura's mana value" (CR 603.2, 303.4, 202.3): an Aura this ability's controller controls became attached - entering
/// attached as an Aura spell resolves, or moved - to a permanent that is not a land and that an opponent of theirs controls,
/// both read as they are once it is attached. About that permanent; the Aura rides in the data ("that Aura").
fn aura_you_control_becomes_attached_to_a_cheaper_opponents_permanent(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Attached { attachment: aura, to: Target::Object(host) } = *event else { return None };
    let theirs = q.state().opponents(me.controller).any(|p| p == q.controller(host));
    let qualifies = q.is_on_battlefield(host)
        && q.chars(aura).has_subtype(q.words(me.source).subtype(Subtype::Aura))
        && q.controller(aura) == me.controller
        && theirs
        && !q.chars(host).is(CardType::Land)
        && q.mana_value(host) <= q.mana_value(aura);
    qualifies.then_some(Fired { subject: Target::Object(host), data: [aura.0, u32::MAX] })
}

/// "Gain control of that permanent for as long as that Aura is attached to it" (CR 613.1b, 611.2b): the rules' source of the
/// effect is this ability (CR 113.7) and no rule reads a control effect's source, so the Aura is recorded as the effect's
/// source object only as the handle its duration follows: it ends the moment that Aura is no longer attached to that
/// permanent - and never begins if it is not by the time the ability resolves (`Duration::AsLongAs`). Eriette leaving does not
/// end it (ruling 2024-04-12).
fn gain_control_while_that_aura_is_attached(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(permanent) = item.subject else { return Ok(()) };
    let aura = ObjectId(item.data[0]);
    g.gain_control(aura, permanent, item.controller, Duration::AsLongAs(g.card_id(&CARD), 0));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACHED, aura_you_control_becomes_attached_to_a_cheaper_opponents_permanent),
        gain_control_while_that_aura_is_attached,
        "Whenever an Aura you control becomes attached to a nonland permanent an opponent controls with mana value less than or equal to that Aura's mana value, gain control of that permanent for as long as that Aura is attached to it.",
    )],
    lasting: &[Lasting { text: "that Aura is attached to it", holds: source_remains_attached_to_it }],
    ..oracle::ERIETTE_THE_BEGUILER
};
