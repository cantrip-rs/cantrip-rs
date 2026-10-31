use cantrip_api::*;
use cantrip_mechanics::{library, select};
use cantrip_mechanics::statics::CREATURES_YOU_CONTROL_GET_PLUS_ONE;
use cantrip_mechanics::tokens::{self, GNOME};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::craft;
use cantrip_oracle as oracle;

/// "Search your library for a basic Plains card, reveal it, put it into your hand, then shuffle. You gain 2 life." The
/// Plains is a basic land type read through the Bricks' words (CR 612); a search for a quality may find nothing (CR
/// 701.23b), and the life is gained either way.
fn fetch_plains(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_hand(g, item.controller, item.object, select::BASIC_PLAINS_CARD, true)?;
    g.gain_life(item.controller, 2);
    Ok(())
}

/// "When this artifact enters, create two 1/1 colorless Gnome artifact creature tokens."
fn two_gnomes(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &GNOME, 2)?;
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, two_gnomes, "When this artifact enters, create two 1/1 colorless Gnome artifact creature tokens.")],
    statics: &[CREATURES_YOU_CONTROL_GET_PLUS_ONE],
    ..oracle::COSMIUM_KILN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, fetch_plains, "When this artifact enters, search your library for a basic Plains card, reveal it, put it into your hand, then shuffle. You gain 2 life."),
        craft::ability(mana("{5}{W}{W}"), &craft::ARTIFACT, "Craft with artifact {5}{W}{W}"),
    ],
    back: Some(&BACK),
    ..oracle::CLAY_FIRED_BRICKS
};

pub static EXTRA: &[&CardDef] = &[&GNOME];
