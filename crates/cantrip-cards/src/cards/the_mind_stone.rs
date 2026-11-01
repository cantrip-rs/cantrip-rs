use cantrip_api::*;
use cantrip_mechanics::{harness, targets};
use cantrip_oracle as oracle;

/// Exile it, then return that card to the battlefield under its owner's control: a new object (CR 400.7).
fn blink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    cantrip_mechanics::exile_and_return(g, &[permanent])
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::W), "{T}: Add {W}."),
        harness::ability(
            cost("{5}{W}, {T}"),
            "{5}{W}, {T}: Harness The Mind Stone.",
        ),
    ],
    delayed: &[triggered(cantrip_mechanics::triggers::YOUR_END_STEP, blink, "At the beginning of your end step, exile up to one other target nonland permanent you control, then return that card to the battlefield under its owner's control.")
        .targets(&[up_to_one(targets::other_nonland_permanent_you_control_filter)])],
    statics: &[harness::infinity(
        "\u{221e} \u{2014} At the beginning of your end step, exile up to one other target nonland permanent you control, then return that card to the battlefield under its owner's control.",
    )],
    ..oracle::THE_MIND_STONE
};
