use cantrip_api::*;
use cantrip_mechanics::{effects, pump, tokens, triggers::THIS_ENTERS, untapped_creature_you_control};
use cantrip_oracle as oracle;

/// "Vivid - ... where X is the number of colors among permanents you control" (CR 105.2, 207.2c): each of the five colors any
/// permanent they control has, counted once however many have it; colorless is not a color. Read through the layers as the
/// ability resolves, so X is fixed then (the ruling), the Kithkeeper itself counting while it is on the battlefield.
fn colors_among_permanents_you_control(q: &dyn Query, player: PlayerId) -> u32 {
    let mut colors = ColorSet::NONE;
    for id in q.permanents_of(player) {
        colors = ColorSet(colors.0 | q.chars(id).colors.0);
    }
    colors.iter().count() as u32
}

/// "Create X 1/1 green and white Kithkin creature tokens, where X is the number of colors among permanents you control."
fn create_kithkin_for_each_color(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = colors_among_permanents_you_control(g, item.controller);
    tokens::create(g, item.controller, &tokens::KITHKIN, x)?;
    Ok(())
}

/// "This creature gets +3/+0 and gains flying until end of turn" (CR 611.2a, 702.9b): one instruction.
fn gets_plus_three_and_flies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        pump(g, item.object, item.object, 3, 0);
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Flying);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            create_kithkin_for_each_color,
            "Vivid \u{2014} When this creature enters, create X 1/1 green and white Kithkin creature tokens, where X is the number of colors among permanents you control.",
        ),
        ability(
            tap_untapped(3, untapped_creature_you_control),
            gets_plus_three_and_flies,
            "Tap three untapped creatures you control: This creature gets +3/+0 and gains flying until end of turn.",
        ),
    ],
    ..oracle::KITHKEEPER
};

pub static EXTRA: &[&CardDef] = &[&tokens::KITHKIN];
