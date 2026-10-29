use cantrip_api::*;
use cantrip_mechanics::{counter_unless_pays, effects, targets};
use cantrip_oracle as oracle;

/// "Counter target spell unless its controller pays {3}." (CR 118.12, 701.6)
fn interfere(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_unless_pays(g, item, 0, mana("{3}"))?;
    Ok(())
}

/// "Tap up to one target creature and put a stun counter on it. Draw a card." (CR 701.26a, 122.1d, 121.1)
fn observe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_and_stun::<1>(g, item)?;
    g.draw(item.controller, 1);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(interfere, "Interfere — Counter target spell unless its controller pays {3}.")
        .targets(&[targets::SPELL]),
    mode(observe, "Observe — Tap up to one target creature and put a stun counter on it. Draw a card.")
        .targets(&[targets::CREATURE.optional()]),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    ..oracle::CONSIDER_THE_PRIME_DIRECTIVE
};
