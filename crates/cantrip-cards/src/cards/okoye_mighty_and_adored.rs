use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::{delayed, targets, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature. Whenever that creature attacks the monarch this turn, it gains double
/// strike and trample until end of turn": the delayed ability (CR 603.7b) watches that creature, so its trigger asks only
/// whether that creature attacked the monarch - Okoye herself too, when she is the target; its source is Okoye (CR 603.7e).
fn vigil(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    delayed::this_turn_watching(g, item, creature);
    Ok(())
}

/// "Whenever that creature attacks the monarch": the defending player is the monarch (CR 725.1); a planeswalker the
/// monarch controls is not the monarch.
fn attacks_the_monarch(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defender: Target::Player(p), .. } if *attacker == source && q.state().player(*p).monarch => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "It gains double strike and trample until end of turn": "it" is the creature watched, the ability's subject.
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    if targets::permanent(g, creature) {
        g.add_effect(
            item.object,
            creature,
            EffectKind::AddKeywords(Keywords::of(&[Keyword::DoubleStrike, Keyword::Trample])),
            Duration::EndOfTurn,
        );
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::become_monarch, "When Okoye enters, you become the monarch."),
        triggered(triggers::BEGINNING_OF_YOUR_COMBAT, vigil, "At the beginning of combat on your turn, put a +1/+1 counter on target creature. Whenever that creature attacks the monarch this turn, it gains double strike and trample until end of turn.")
            .targets(&[targets::CREATURE]),
    ],
    delayed: &[triggered(when(EventMask::ATTACKS, attacks_the_monarch), rally, "Whenever that creature attacks the monarch this turn, it gains double strike and trample until end of turn.")],
    ..oracle::OKOYE_MIGHTY_AND_ADORED
};
