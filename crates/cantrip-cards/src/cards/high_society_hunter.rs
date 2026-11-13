use cantrip_api::*;
use cantrip_mechanics::{effects, select, triggers};
use cantrip_oracle as oracle;

/// "You may sacrifice another creature. If you do, put a +1/+1 counter on this creature." (CR 608.2c-d, 701.21a): the sacrifice is
/// chosen as the ability resolves and may be declined; the counter comes only if a creature was actually sacrificed.
fn sacrifice_to_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, select::another_creature_permanents(select::up_to(1)))?.is_empty() {
        effects::plus_one_counter_on_this(g, item)?;
    }
    Ok(())
}

/// "Whenever another nontoken creature dies" (CR 700.4, 603.10a, 111.7): `triggers::another_creature_dies` for a creature that
/// was not a token, looked back on as it last was on the battlefield.
fn another_nontoken_creature_dies(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, .. } if q.obj(*old).token => None,
        _ => triggers::another_creature_dies(q, source, event),
    }
}

// Flying is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS,
            sacrifice_to_grow,
            "Whenever this creature attacks, you may sacrifice another creature. If you do, put a +1/+1 counter on this creature.",
        ),
        triggered(when(EventMask::ZONE_CHANGE, another_nontoken_creature_dies), |g, item| effects::draw(g, item, 1), "Whenever another nontoken creature dies, draw a card."),
    ],
    ..oracle::HIGH_SOCIETY_HUNTER
};
