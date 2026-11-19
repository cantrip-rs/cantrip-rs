use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Draw three cards. Create a 1/1 blue Faerie creature token with flying and 'This token can block only creatures with flying.'"
/// (CR 121.1, 111.1, 608.2c): two instructions, in order.
fn draw_three_then_create_a_faerie(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 3)?;
    g.end_sentence();
    tokens::create(g, item.controller, &tokens::FAERIE_THAT_BLOCKS_ONLY_FLIERS, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], draw_three_then_create_a_faerie), ..oracle::INTO_THE_FAE_COURT };

pub static EXTRA: &[&CardDef] = &[&tokens::FAERIE_THAT_BLOCKS_ONLY_FLIERS];
