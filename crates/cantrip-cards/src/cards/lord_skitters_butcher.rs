use cantrip_api::*;
use cantrip_mechanics::{effects, select, tokens, triggers};
use cantrip_oracle as oracle;

/// "You may sacrifice another creature. If you do, scry 2, then draw a card." (CR 701.21a, 608.2c, 701.22): the creature is chosen as
/// the ability resolves, not as a cost; the scry and the draw happen only if one was actually sacrificed.
fn sacrifice_another_to_scry_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let sacrificed = effects::may_sacrifice(g, item.controller, item.object, select::another_creature_permanents(select::up_to(1)))?;
    if sacrificed.is_empty() {
        return Ok(());
    }
    g.end_sentence();
    effects::scry_then_draw::<2>(g, item)
}

static MODES: ModalDef = modes(&[
    mode(tokens::create_rats_that_cant_block::<1>, "Create a 1/1 black Rat creature token with \"This token can't block.\""),
    mode(sacrifice_another_to_scry_and_draw, "You may sacrifice another creature. If you do, scry 2, then draw a card."),
    mode(
        effects::creatures_you_control_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Menace]).0 }>,
        "Creatures you control gain menace until end of turn.",
    ),
]);

/// "When this creature enters, choose one —" (CR 700.2, 603.3c): the mode is chosen as the ability is put on the stack.
pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_ENTERS,
        &MODES,
        "When this creature enters, choose one — Create a 1/1 black Rat creature token with \"This token can't block\"; or you may sacrifice another creature. If you do, scry 2, then draw a card; or creatures you control gain menace until end of turn.",
    )],
    ..oracle::LORD_SKITTERS_BUTCHER
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
