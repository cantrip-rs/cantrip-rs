use cantrip_api::*;
use cantrip_mechanics::{flurry, tokens};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Mode 0: "Create two 1/1 white Human Soldier creature tokens", together (CR 101.4).
fn make_tokens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::HUMAN_SOLDIER, 2)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(make_tokens, "Create two 1/1 white Human Soldier creature tokens."),
    mode(effects::plus_one_counter_on_each_creature_you_control, "Put a +1/+1 counter on each creature you control."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[flurry::modal_trigger(&MODAL, "Whenever you cast your second spell each turn, choose one \u{2014} Create two 1/1 white Human Soldier creature tokens.; or Put a +1/+1 counter on each creature you control.")],
    ..oracle::COSMOGRAND_ZENITH
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN_SOLDIER];
