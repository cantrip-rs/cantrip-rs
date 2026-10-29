use cantrip_api::*;
use cantrip_mechanics::subtype_cards_in_graveyard;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Mill three cards. Then if there is an Elf card in your graveyard, each opponent loses 2 life and you gain 2 life" (CR 701.17,
/// 608.2c): the graveyard is looked at after the mill, the milled cards included.
fn mill_three_then_drain_if_an_elf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.mill(item.controller, 3);
    let elf = g.words(item.object).subtype(Subtype::Elf);
    if subtype_cards_in_graveyard(g, item.controller, elf) > 0 {
        g.each_opponent_loses_life(item.controller, 2);
        g.gain_life(item.controller, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        mill_three_then_drain_if_an_elf,
        "When this creature enters, mill three cards. Then if there is an Elf card in your graveyard, each opponent loses 2 life and you gain 2 life.",
    )],
    ..oracle::DAWNHAND_EULOGIST
};
