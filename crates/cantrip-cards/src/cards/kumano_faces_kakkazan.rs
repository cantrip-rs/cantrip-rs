use cantrip_api::*;
use cantrip_mechanics::exile_and_return_transformed;
use cantrip_mechanics::next_creature_counter;
use cantrip_oracle as oracle;

/// I: "This Saga deals 1 damage to each opponent and each planeswalker they control." The Saga is the source (CR 120.2b); all of it is one
/// simultaneous event.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let walkers: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Planeswalker) && opponents.contains(&g.controller(id)))
        .collect();
    let hits: Vec<(ObjectId, Target, i64)> = opponents
        .iter()
        .map(|&p| Target::Player(p))
        .chain(walkers.into_iter().map(Target::Object))
        .map(|target| (item.object, target, 1))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

/// III: "Exile this Saga, then return it to the battlefield transformed under your control."
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_and_return_transformed(g, item.object, item.controller)
}

/// "If a creature dealt damage this turn by a source you controlled would die, exile it instead" (CR 614.1a, 700.4): a
/// creature moving from the battlefield to a graveyard that some source, controlled by this effect's controller as it
/// dealt the damage, dealt damage to this turn.
fn damaged_by_your_source_would_die(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    z.from.is(ZoneKind::Battlefield)
        && z.to.is(ZoneKind::Graveyard)
        && q.chars(z.object).is(CardType::Creature)
        && q.history().select(EventQuery::kind(EventMask::DAMAGE).this_turn()).any(|r| {
            matches!(r.event, Event::Damage { target: Target::Object(o), controller, .. }
                if o == z.object && controller == asker.controller)
        })
}

pub static BACK: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        damaged_by_your_source_would_die,
        cantrip_mechanics::replacements::exile_instead_apply,
        "exiles a creature dealt damage this turn by a source you controlled instead of letting it die",
    )],
    ..oracle::ETCHING_OF_KUMANO
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} This Saga deals 1 damage to each opponent and each planeswalker they control."),
        chapter(2, next_creature_counter::chapter, "II \u{2014} When you next cast a creature spell this turn, that creature enters with an additional +1/+1 counter on it."),
        chapter(3, chapter_three, "III \u{2014} Exile this Saga, then return it to the battlefield transformed under your control."),
    ],
    delayed: &[next_creature_counter::DELAYED],
    granted_replacements: &[next_creature_counter::ENTERS_WITH_A_COUNTER.text("that creature enters with an additional +1/+1 counter on it.")],
    back: Some(&BACK),
    ..oracle::KUMANO_FACES_KAKKAZAN
};
