use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "When this creature enters, draw a card for each other Dinosaur you control" (CR 121.1, 608.2c): the Dinosaurs its
/// controller controls as the ability resolves, not counting the Dreadmaw itself, all drawn together; the type is read
/// through the words of the Dreadmaw's text (CR 612) and through the layers, so a changeling counts.
fn draw_for_each_other_dinosaur(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let dinosaur = g.words(item.object).subtype(Subtype::Dinosaur);
    let others = g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).has_subtype(dinosaur)).count();
    g.draw(item.controller, others as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        draw_for_each_other_dinosaur,
        "When this creature enters, draw a card for each other Dinosaur you control.",
    )],
    ..oracle::EARTHSHAKER_DREADMAW
};
