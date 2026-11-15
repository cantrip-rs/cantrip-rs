use cantrip_api::*;
use cantrip_mechanics::{craft, effects, targets};
use cantrip_oracle as oracle;

/// "Tap another target nonland permanent. Its activated abilities can't be activated for as long as it remains tapped"
/// (CR 701.26a, 602.5, 611.2b). A resolved effect, so it lasts after the Net is gone - crafted into Braided Quipu, say - and
/// ends the moment the permanent untaps; a permanent already tapped is netted too (the rulings). Every activated
/// ability, mana abilities and those whose cost untaps it included. Nothing if the target is no longer legal (CR 608.2b).
fn net(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(permanent);
    g.add_effect(item.object, permanent, EffectKind::CantActivateAbilities, Duration::WhileTapped);
    Ok(())
}

/// "Draw a card for each artifact you control, then put this artifact into its owner's library third from the top"
/// (CR 401.1): the artifacts counted as the ability resolves, Braided Quipu among them while it is still there (CR 608.2h);
/// a library of two or fewer cards takes it at the bottom once the cards are drawn (the ruling). A Quipu that has left the
/// battlefield is a new object the ability does not move (CR 400.7).
fn draw_and_tuck(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let artifacts = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Artifact)).count() as u32;
    g.draw(you, artifacts);
    if targets::permanent(g, item.object) {
        g.put_into_library(&[item.object], LibraryPlace::FromTop(2));
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{U}").and(TAP),
        draw_and_tuck,
        "{3}{U}, {T}: Draw a card for each artifact you control, then put this artifact into its owner's library third from the top.",
    )],
    ..oracle::BRAIDED_QUIPU
};

pub static CARD: CardDef = CardDef {
    // "This artifact enters with three net counters on it" (CR 122.6, 614.1c).
    enters: Some(effects::enters_with_counters::<{ CounterKind::Net as usize }, 3>),
    abilities: &[
        ability(
            TAP.and(remove_counters(CounterKind::Net, 1)),
            net,
            "{T}, Remove a net counter from this artifact: Tap another target nonland permanent. Its activated abilities can't be activated for as long as it remains tapped.",
        )
        .targets(&[target(targets::other_nonland_permanent_filter)]),
        craft::ability(mana("{1}{U}"), &craft::ARTIFACT, "Craft with artifact {1}{U}"),
    ],
    back: Some(&BACK),
    ..oracle::BRAIDED_NET
};
