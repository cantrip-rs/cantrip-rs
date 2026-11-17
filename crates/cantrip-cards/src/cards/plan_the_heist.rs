use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

/// "Surveil 3 if you have no cards in hand. Then draw three cards.": the hand is looked at as the spell resolves
/// (CR 608.2c), and the draw happens either way.
fn surveil_if_empty_handed_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.state().zone(Zone::hand(item.controller)).is_empty() {
        effects::surveil(g, item, 3)?;
    }
    g.end_sentence();
    effects::draw(g, item, 3)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], surveil_if_empty_handed_then_draw),
    abilities: &[plot(
        mana("{3}{U}"),
        "Plot {3}{U} (You may pay {3}{U} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    ..oracle::PLAN_THE_HEIST
};
