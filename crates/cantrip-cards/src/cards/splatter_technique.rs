use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw four cards." (CR 121.1)
fn draw_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 4);
    Ok(())
}

/// "Splatter Technique deals 4 damage to each creature and planeswalker." All at once (CR 120.3, 608.2c).
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature_and_planeswalker(g, item, 4);
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(draw_four, "Draw four cards."),
    mode(damage, "Splatter Technique deals 4 damage to each creature and planeswalker."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SPLATTER_TECHNIQUE };
