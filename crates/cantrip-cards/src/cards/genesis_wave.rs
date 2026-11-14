use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_mechanics::select::{self, PERMANENT_CARD};
use cantrip_oracle as oracle;

/// "Reveal the top X cards of your library. You may put any number of permanent cards with mana value X or less from among
/// them onto the battlefield. Then put all cards revealed this way that weren't put onto the battlefield into your
/// graveyard." Each card is judged alone for "mana value X or less" (CR 202.3e; the ruling: it is not a total), and X is
/// the number chosen as it was cast (CR 107.3), so X = 0 reveals nothing (CR 701.20).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::Selection::new(select::ANY_NUMBER).with(PERMANENT_CARD).with(select::mana_value_at_most(item.x)).called("permanent");
    library::reveal_top_select_onto_battlefield_rest_to_graveyard(g, item.controller, item.object, item.x as usize, rule, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ..oracle::GENESIS_WAVE
};
