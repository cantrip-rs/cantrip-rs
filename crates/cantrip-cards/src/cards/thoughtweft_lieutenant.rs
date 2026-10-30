use cantrip_api::*;
use cantrip_mechanics::{pump, targets, trample_until_end_of_turn};
use cantrip_oracle as oracle;

/// "Whenever this creature or another Kithkin you control enters" (CR 603.6a): this permanent, or another Kithkin - a token too -
/// under your control as it entered. "Kithkin" is a word of its text (CR 612).
fn this_or_another_kithkin_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if to.is(ZoneKind::Battlefield)
                && (*new == me.source
                    || (q.controller(*new) == me.controller && q.chars(*new).has_subtype(q.words(me.source).subtype(Subtype::Kithkin)))) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Target creature you control gets +1/+1 and gains trample until end of turn."
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 1, 1);
        trample_until_end_of_turn(g, item.object, creature);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, this_or_another_kithkin_enters),
        rally,
        "Whenever this creature or another Kithkin you control enters, target creature you control gets +1/+1 and gains trample until end of turn.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::THOUGHTWEFT_LIEUTENANT
};
