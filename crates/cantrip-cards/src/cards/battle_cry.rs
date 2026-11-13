use cantrip_api::*;
use cantrip_mechanics::pump;
use cantrip_oracle as oracle;

/// Each creature that begins blocking triggers once, even if it blocks several attackers (CR 509.3a).
fn creature_blocks(_: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Blocks { blocker, declared: true, .. } => Fired::about(Target::Object(*blocker)),
        _ => None,
    }
}

/// The creature that blocked gets +0/+1 until end of turn (CR 611.2a).
fn bolster_blocker(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(blocker) = item.subject.object() {
        pump(g, item.object, blocker, 0, 1);
    }
    Ok(())
}

/// Untap all white creatures you control, then watch for creatures blocking this turn (CR 701.26b, 603.7b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let white = g.words(item.object).color(Color::White);
    let creatures: Vec<_> = g.permanents_of(item.controller).into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.chars(id).colors.has(white))
        .collect();
    g.untap_together(&creatures);
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::ThisTurn,
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    delayed: &[
        triggered(
            when_asked(EventMask::BLOCKS, creature_blocks),
            bolster_blocker,
            "Whenever a creature blocks this turn, it gets +0/+1 until end of turn.",
        ),
    ],
    ..oracle::BATTLE_CRY
};
