use cantrip_api::*;
use cantrip_mechanics::{blight, library, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "Whenever a creature you control with one or more counters on it dies" (CR 700.4, 603.10a): looked back on, so the creature
/// is judged as it last was on the battlefield, under the controller it last had, with every counter of every kind it had.
/// The trigger carries how many ("that many cards").
fn creature_with_counters_dies(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::ZoneChange { old, from, to, .. } = event else { return None };
    if !from.is(ZoneKind::Battlefield) || !to.is(ZoneKind::Graveyard) || !q.chars(*old).is(CardType::Creature) || q.obj(*old).controller != me.controller {
        return None;
    }
    let counters: u32 = q.obj(*old).counter_array().iter().map(|&c| c.max(0) as u32).sum();
    (counters > 0).then_some(Fired { subject: Target::None, data: [counters, 0] })
}

/// "Exile that many cards from the top of your library. Until your next end step, you may play those cards."
fn exile_and_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::exile_top_play_until_your_next_end_step(g, item.controller, item.data[0] as usize);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            |g, item| blight::in_effect(g, item.controller, item.object, 1).map(|_| ()),
            "Whenever this creature attacks, blight 1. (Put a -1/-1 counter on a creature you control.)",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, creature_with_counters_dies),
            exile_and_play,
            "Whenever a creature you control with one or more counters on it dies, exile that many cards from the top of your library. Until your next end step, you may play those cards.",
        ),
    ],
    ..oracle::SHADOW_URCHIN
};
