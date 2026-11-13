use cantrip_api::*;
use cantrip_mechanics::{library, untap};
use cantrip_oracle as oracle;

/// "Draw X cards. If X is 10 or more, instead shuffle your graveyard into your library, draw X cards, untap up to five lands,
/// and you have no maximum hand size for the rest of the game. Exile Finale of Revelation."
///
/// X is the value chosen as the spell was cast (CR 107.3a). The graveyard shuffled in is the one the spell finds as it
/// resolves, which does not hold the spell itself, still on the stack; a library is shuffled even when there is no graveyard
/// to put in it (CR 701.24d). The exile is the spell's last instruction, so a countered Finale goes to the graveyard instead.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if item.x >= 10 {
        library::shuffle_graveyard_into_library(g, you);
        g.draw(you, item.x as u32);
        untap::up_to_lands(g, item.object, you, 5)?;
        g.add_effect(item.object, item.object, EffectKind::MaximumHandSize(None), Duration::Indefinite);
    } else {
        g.draw(you, item.x as u32);
    }
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::FINALE_OF_REVELATION };
