use cantrip_api::*;
use cantrip_mechanics::{effects, hand, targets};
use cantrip_oracle as oracle;

/// The counter is put on the same artifact, if it is still there (CR 608.2h, 122.6).
fn put_hatchling_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Hatchling, 1);
    Ok(())
}

/// You may put a creature card from your hand onto the battlefield (CR 608.2d).
fn put_creature_from_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::may_put_onto_battlefield(g, item.controller, item.object, |c| c.is(CardType::Creature))?;
    Ok(())
}

fn has_two_hatchling_counters(q: &dyn Query, asker: &Asker) -> bool {
    q.obj(asker.source).counters(CounterKind::Hatchling) >= 2
}

static HATCH_MODES: ModalDef = modes(
    &[
        mode(put_creature_from_hand, "You may put a creature card from your hand onto the battlefield."),
        mode(effects::return_first_target_to_battlefield, "Return target creature card from your graveyard to the battlefield.")
            .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
    ],
).choose(1, 1);

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{3}").and(TAP), put_hatchling_counter, "{3}, {T}: Put a hatchling counter on this artifact."),
        modal_ability(
            SACRIFICE_THIS,
            &HATCH_MODES,
            "Sacrifice this artifact: Choose one. Activate only if there are two or more hatchling counters on this artifact. You may put a creature card from your hand onto the battlefield; or Return target creature card from your graveyard to the battlefield.",
        ).condition(has_two_hatchling_counters),
    ],
    ..oracle::TRIASSIC_EGG
};
