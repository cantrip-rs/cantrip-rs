use cantrip_api::*;
use cantrip_mechanics::{effects, mana, targets};
use cantrip_oracle as oracle;

/// The tapped 4/4 green Beast creature token it makes.
pub static BEAST: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    ..token!(Beast)
};

/// "Whenever you sacrifice a land": a land of yours moved from the battlefield to a graveyard as a sacrifice.
fn sacrificed_a_land(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, reason: ZoneReason::Sacrifice, .. }
            if from.is(ZoneKind::Battlefield) && q.controller(*old) == q.controller(source) && q.chars(*old).is(CardType::Land) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

fn wake(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let beast = g.card_id(&BEAST);
    g.create_tokens(beast, item.controller, 1, true)?;
    g.untap(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_with_stun_counters::<6>),
    abilities: &[
        triggered(when(EventMask::ZONE_CHANGE, sacrificed_a_land), wake, "Whenever you sacrifice a land, create a tapped 4/4 green Beast creature token and untap this creature.")
            .on_sacrifice(),
        ability(cost("{4}").and(sacrifice(1, targets::filters::land)), |g, item| effects::gain_life(g, item, 2), "{4}, Sacrifice a land: You gain 2 life."),
    ],
    ..oracle::BALOTH_PRIME
};

pub static EXTRA: &[&CardDef] = &[&BEAST];
