use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_mechanics::adapt;
use cantrip_oracle as oracle;

/// Exile up to one target card from a graveyard: it is exiled with this creature (CR 607.2a), for its other ability.
fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = targets::object(g, item, 0) {
        g.exile_with(&[card], g.linked(item), false);
    }
    Ok(())
}

/// Adapt 2 (CR 701.46a).
fn adapt(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    adapt::adapt(g, item.object, 2);
    Ok(())
}

/// Put a creature card exiled with this creature onto the battlefield under your control with a finality counter on it. It
/// gains haste. Sacrifice it at the beginning of the next end step. A lone card is no choice; several are asked.
fn reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let eligible: Vec<ObjectId> =
        g.exiled_with(g.linked(item)).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let chosen = if eligible.len() == 1 {
        eligible.first().copied()
    } else {
        choose::card(g, item.controller, item.object, Purpose::PutOntoBattlefield, eligible)?
    };
    let Some(chosen) = chosen else { return Ok(()) };
    // The finality counter is on it as it enters (CR 122.6, 614.1c), not put on it afterwards.
    let returned = g.put_onto_battlefield_with_counters(chosen, item.controller, false, &[(CounterKind::Finality, 1)])?;
    if !targets::permanent(g, returned) {
        return Ok(());
    }
    cantrip_mechanics::effects::gain_haste(g, item.object, returned);
    cantrip_mechanics::delayed::at_next_end_step(g, item, 0, Target::Object(returned));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT, exile_it, "At the beginning of combat on your turn, exile up to one target card from a graveyard.")
            .targets(&[cantrip_mechanics::targets::CARD_IN_A_GRAVEYARD.optional()]),
        ability(cost("{1}{B}"), adapt, "{1}{B}: Adapt 2."),
        triggered(cantrip_mechanics::triggers::PLUS_ONE_COUNTERS_PUT_ON_THIS, reanimate, "Whenever one or more +1/+1 counters are put on this creature, put a creature card exiled with this creature onto the battlefield under your control with a finality counter on it. It gains haste. Sacrifice it at the beginning of the next end step."),
    ],
    delayed: &[delayed_trigger(cantrip_mechanics::delayed::sacrifice_subject, "Sacrifice it.")],
    ..oracle::EMPEROR_OF_BONES
};
