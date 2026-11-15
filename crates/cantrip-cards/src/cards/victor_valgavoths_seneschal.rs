use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// What the trigger does depends on how many times this ability of this Victor has resolved this turn before: this turn's
/// history counts them. First, surveil 2. Second, each opponent discards a card. Third, a creature card from a graveyard
/// comes back under its controller's control. A fourth and later resolution does nothing.
fn eerie(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    match g.history().resolutions(item.object, item.ability) {
        0 => effects::surveil(g, item, 2),
        1 => effects::each_opponent_discards_a_card(g, item),
        2 => effects::put_a_creature_card_from_a_graveyard_onto_the_battlefield(g, item).map(|_| ()),
        _ => Ok(()),
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EERIE,
        eerie,
        "Eerie \u{2014} Whenever an enchantment you control enters and whenever you fully unlock a Room, surveil 2 if this is the first time this ability has resolved this turn. If it's the second time, each opponent discards a card. If it's the third time, put a creature card from a graveyard onto the battlefield under your control.",
    )],
    ..oracle::VICTOR_VALGAVOTHS_SENESCHAL
};
