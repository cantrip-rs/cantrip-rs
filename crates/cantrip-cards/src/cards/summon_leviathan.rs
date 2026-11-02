use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "I — Return each creature that isn't a Kraken, Leviathan, Merfolk, Octopus, or Serpent to its owner's hand." (CR 608.2, 400.7).
fn i_return_creature_isnt_kraken(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && !((q
                        .chars(id)
                        .has_subtype(q.words(item.object).subtype(Subtype::Kraken))
                        || q
                            .chars(id)
                            .has_subtype(
                                q.words(item.object).subtype(Subtype::Leviathan),
                            )
                        || q
                            .chars(id)
                            .has_subtype(q.words(item.object).subtype(Subtype::Merfolk))
                        || q
                            .chars(id)
                            .has_subtype(q.words(item.object).subtype(Subtype::Octopus))
                        || q
                            .chars(id)
                            .has_subtype(
                                q.words(item.object).subtype(Subtype::Serpent),
                            )))
            })
            .collect::<Vec<ObjectId>>()
    };
    g.return_to_hand(&objects_0);
    Ok(())
}

/// "Until end of turn" (CR 603.2).
fn watch_3_event(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Attacks { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    if q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Kraken))
        || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Leviathan))
        || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Merfolk))
        || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Octopus))
        || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Serpent))
    {
        return Fired::about(subject);
    }
    None
}

/// "Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card." (CR 608.2, 121.1).
fn watch_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// "II, III — Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card." (CR 608.2).
fn ii_iii_whenever_kraken_leviathan(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::ThisTurn,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

/// "Until end of turn" (CR 603.2).
fn watch_7_event(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Attacks { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    if q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Kraken))
        || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Leviathan))
        || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Merfolk))
        || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Octopus))
        || q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Serpent))
    {
        return Fired::about(subject);
    }
    None
}

/// "Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card." (CR 608.2, 121.1).
fn watch_7(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// "II, III — Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card." (CR 608.2).
fn ii_iii_whenever_kraken_leviathan_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::ThisTurn,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 1),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(
            1,
            i_return_creature_isnt_kraken,
            "I — Return each creature that isn't a Kraken, Leviathan, Merfolk, Octopus, or Serpent to its owner's hand.",
        ),
        chapter(
            2,
            ii_iii_whenever_kraken_leviathan,
            "II, III — Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card.",
        ),
        chapter(
            3,
            ii_iii_whenever_kraken_leviathan_2,
            "II, III — Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card.",
        ),
        triggers::ward::<2>("Ward {2}"),
    ],
    delayed: &[
        triggered(
            when_asked(EventMask::ATTACKS, watch_3_event),
            watch_3,
            "Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card.",
        ),
        triggered(
            when_asked(EventMask::ATTACKS, watch_7_event),
            watch_7,
            "Until end of turn, whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks, draw a card.",
        ),
    ],
    ..oracle::SUMMON_LEVIATHAN
};
