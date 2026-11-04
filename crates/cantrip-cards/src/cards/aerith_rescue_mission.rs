use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets, tokens};
use cantrip_oracle as oracle;

/// "Take the Elevator — Create three 1/1 colorless Hero creature tokens." (CR 701.7, 111.1): together (CR 101.4).
fn elevator(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::HERO_1_1_COLORLESS, 3)?;
    Ok(())
}

/// "Take 59 Flights of Stairs — Tap up to three target creatures. Put a stun counter on one of them." (CR 701.26a, 122.1d,
/// 608.2d): the creatures still legal targets are tapped together; the controller chooses one of those as the spell resolves,
/// tapped already or not, and it gets the stun counter.
fn stairs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    g.tap_together(&creatures);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, creatures, select::cards(select::exactly(1)))?;
    let Some(&stunned) = chosen.first() else { return Ok(()) };
    g.add_counters(stunned, CounterKind::Stun, 1);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(elevator, "Take the Elevator — Create three 1/1 colorless Hero creature tokens."),
    mode(stairs, "Take 59 Flights of Stairs — Tap up to three target creatures. Put a stun counter on one of them.")
        .targets(&[targets::CREATURE.between(0, 3)]),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::AERITH_RESCUE_MISSION
};

pub static EXTRA: &[&CardDef] = &[&tokens::HERO_1_1_COLORLESS];
