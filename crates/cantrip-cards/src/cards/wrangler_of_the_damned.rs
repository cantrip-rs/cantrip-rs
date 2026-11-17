use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{history, tokens};
use cantrip_oracle as oracle;

fn create_a_spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::WHITE_FLYING_SPIRIT_2_2, 1)?;
    Ok(())
}

/// "At the beginning of your end step, if you haven't cast a spell from your hand this turn, create a 2/2 white Spirit creature token
/// with flying." (CR 603.4, 513.1): an intervening "if" checked as the step begins and again as the ability resolves.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_END_STEP,
        create_a_spirit,
        "At the beginning of your end step, if you haven't cast a spell from your hand this turn, create a 2/2 white Spirit creature token with flying.",
    )
    .condition(history::you_havent_cast_a_spell_from_your_hand)],
    ..oracle::WRANGLER_OF_THE_DAMNED
};

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_FLYING_SPIRIT_2_2];
