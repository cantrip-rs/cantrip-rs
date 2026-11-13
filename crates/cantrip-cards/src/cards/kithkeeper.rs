use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers::THIS_ENTERS, untapped_creature_you_control, vivid};
use cantrip_oracle as oracle;

/// Read through the layers as the ability resolves, so X is fixed then (the ruling), the Kithkeeper itself counting while it is on
/// the battlefield.
/// "Create X 1/1 green and white Kithkin creature tokens, where X is the number of colors among permanents you control."
fn create_kithkin_for_each_color(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = vivid::count(g, item.controller) as u32;
    tokens::create(g, item.controller, &tokens::KITHKIN, x)?;
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
            effects::this_gets_and_gains_flying::<3, 0>,
            "Tap three untapped creatures you control: This creature gets +3/+0 and gains flying until end of turn.",
        ),
    ],
    ..oracle::KITHKEEPER
};

pub static EXTRA: &[&CardDef] = &[&tokens::KITHKIN];
