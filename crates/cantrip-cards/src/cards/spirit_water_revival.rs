use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// "Draw two cards. If this spell's additional cost was paid, instead shuffle your graveyard into your library, draw seven
/// cards, and you have no maximum hand size for the rest of the game. Exile Spirit Water Revival."
///
/// The additional cost is the card's waterbend `kicker` (CR 701.67): paid, `item.kicked` is 1. The graveyard shuffled in is
/// the one the spell finds as it resolves, which does not hold the spell itself, still on the stack; a library is shuffled
/// even when there is no graveyard to put in it (CR 701.24a). The exile is the spell's last instruction.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if item.kicked > 0 {
        library::shuffle_graveyard_into_library(g, you);
        g.draw(you, 7);
        g.add_effect(item.object, item.object, EffectKind::MaximumHandSize(None), Duration::Indefinite);
    } else {
        g.draw(you, 2);
    }
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    // "As an additional cost to cast this spell, you may waterbend {6}."
    kicker: waterbend_kicker(mana("{6}")),
    ..oracle::SPIRIT_WATER_REVIVAL
};
