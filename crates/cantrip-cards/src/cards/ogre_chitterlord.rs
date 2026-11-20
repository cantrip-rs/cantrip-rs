use cantrip_api::*;
use cantrip_mechanics::{effects, permanents_with_subtype_you_control, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create two 1/1 black Rat creature tokens with 'This token can't block.' Then if you control five or more Rats, each Rat you
/// control gets +2/+0 until end of turn." (CR 608.2c, 611.2c): the Rats are counted after the tokens are made, with the new ones,
/// by the words of this card's text (CR 612); the pump reaches the Rats there are as it resolves, and no Rat that enters later. A
/// Rat that is not a creature counts and is pumped too ("each Rat", CR 205.3).
fn create_rats_then_pump_them_if_five(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_rats_that_cant_block::<2>(g, item)?;
    g.end_sentence();
    let rat = g.words(item.object).subtype(Subtype::Rat);
    let rats = permanents_with_subtype_you_control(g, item.controller, rat);
    if rats >= 5 {
        effects::subtype_you_control_get(g, item, Subtype::Rat, 2, 0);
    }
    Ok(())
}

// Menace is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        create_rats_then_pump_them_if_five,
        "Whenever this creature enters or attacks, create two 1/1 black Rat creature tokens with \"This token can't block.\" Then if you control five or more Rats, each Rat you control gets +2/+0 until end of turn.",
    )],
    ..oracle::OGRE_CHITTERLORD
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
