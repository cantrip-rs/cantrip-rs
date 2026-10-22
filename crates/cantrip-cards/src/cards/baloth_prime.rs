use cantrip_api::*;
use cantrip_mechanics::{targets, effects};
use cantrip_oracle as oracle;

/// "This creature enters tapped with six stun counters on it" (CR 614.1c-d, 122.1d): the counters then keep it tapped
/// through six untaps, unless sacrificing lands untaps it sooner.
fn enters_stunned(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.tapped = true;
    entering.counters[CounterKind::Stun as usize] += 6;
    Ok(())
}

/// The tapped 4/4 green Beast creature token it makes.
pub static BEAST: CardDef = CardDef {
    name: "Beast Token",
    token_name: Some(&[Subtype::Beast]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Beast]),
    power: Some(4),
    toughness: Some(4),
    ..CardDef::BLANK
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
    enters: Some(enters_stunned),
    abilities: &[
        triggered(when(EventMask::ZONE_CHANGE, sacrificed_a_land), wake, "Whenever you sacrifice a land, create a tapped 4/4 green Beast creature token and untap this creature.")
            .on_sacrifice(),
        ability(cost("{4}").and(sacrifice(1, targets::filters::land)), |g, item| effects::gain_life(g, item, 2), "{4}, Sacrifice a land: You gain 2 life."),
    ],
    ..oracle::BALOTH_PRIME
};

pub static EXTRA: &[&CardDef] = &[&BEAST];
