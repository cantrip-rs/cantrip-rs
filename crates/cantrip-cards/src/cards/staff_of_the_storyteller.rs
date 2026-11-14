use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Whenever you create one or more creature tokens" (CR 111.2, 603.2c): a creation, not an entry, so a token that
/// became one as a copy of a permanent spell does not count (CR 111.13). "You create" is the creator, the token's owner.
/// One counter for the tokens one creation made.
fn you_create_creature_tokens(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, reason: ZoneReason::Created, .. }
            if q.obj(*new).owner == q.controller(source) && q.chars(*new).is(CardType::Creature) =>
        {
            Fired::about(Target::Object(*new))
        }
        _ => None,
    }
}

fn story_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Story, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_white_flying_spirit, "When this artifact enters, create a 1/1 white Spirit creature token with flying."),
        triggered(when(EventMask::ZONE_CHANGE, you_create_creature_tokens), story_counter, "Whenever you create one or more creature tokens, put a story counter on this artifact.")
            .once_per_batch(),
        ability(cost("{W}, {T}").and(remove_counters(CounterKind::Story, 1)), |g, item| effects::draw(g, item, 1), "{W}, {T}, Remove a story counter from this artifact: Draw a card."),
    ],
    ..oracle::STAFF_OF_THE_STORYTELLER
};

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_FLYING_SPIRIT];
