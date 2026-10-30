use cantrip_api::*;
use cantrip_mechanics::beam_me_up;
use cantrip_oracle as oracle;

fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], draw),
    ways: &[beam_me_up::way(mana("{2}{U}"), "Beam me up {2}{U}")],
    ..oracle::OPEN_COMMUNICATIONS
};
