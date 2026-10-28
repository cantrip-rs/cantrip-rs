use cantrip_api::*;
use cantrip_mechanics::top_of_library;
use cantrip_oracle as oracle;

/// "Reveal the top card of your library. If it's an artifact card, draw a card." (CR 701.20a, 121.1): the card revealed stays
/// on top unless it is an artifact, in which case it is the card drawn. An empty library reveals nothing and draws nothing.
fn reveal_and_draw_an_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(&top) = top_of_library(g, item.controller, 1).first() else { return Ok(()) };
    g.reveal(item.controller, &[top]);
    if g.chars(top).is(CardType::Artifact) {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        reveal_and_draw_an_artifact,
        "{T}: Reveal the top card of your library. If it's an artifact card, draw a card.",
    )],
    // "You may look at the top card of your library any time" (CR 401.2), which the host shows its controller.
    play_rules: &[PlayRule::LookAtLibraryTop],
    ..oracle::IRON_LAD_DIVERGING_DESTINY
};
