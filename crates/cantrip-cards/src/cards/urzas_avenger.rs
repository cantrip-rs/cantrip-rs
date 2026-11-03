use cantrip_api::*;
use cantrip_mechanics::{choose, effects, pump};
use cantrip_oracle as oracle;

/// The choice is made at resolution, and both changes last until end of turn (CR 608.2d, 611.2a).
fn change_shape(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let chosen = choose::option(g, item.controller, item.object, item.ability.card, 0..4)?;
    let keyword = match chosen {
        0 => Keyword::Banding,
        1 => Keyword::Flying,
        2 => Keyword::FirstStrike,
        _ => Keyword::Trample,
    };
    instruction(g, |g| {
        pump(g, item.object, item.object, -1, -1);
        effects::gain_until_end_of_turn(g, item.object, item.object, keyword);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        Cost::mana(ManaCost::ZERO),
        change_shape,
        "{0}: This creature gets -1/-1 and gains your choice of banding, flying, first strike, or trample until end of turn.",
    )],
    labels: &["Banding", "flying", "first strike", "trample"],
    ..oracle::URZAS_AVENGER
};
