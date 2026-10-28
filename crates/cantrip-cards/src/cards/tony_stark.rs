use cantrip_api::*;
use cantrip_mechanics::library::{Rest, look_at_top};
use cantrip_mechanics::{effects, hand, triggers::BEGINNING_OF_YOUR_COMBAT};
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library. You may reveal an artifact card from among them and put it into your hand.
/// Put the rest on the bottom of your library in a random order." (CR 401.4, 701.20a): a library with fewer cards shows what it
/// has (CR 609.3); with no artifact card among them the cards are still looked at and all go to the bottom.
fn dig_for_an_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top(g, item.controller, item.object, 4, (0, 1), true, |c| c.is(CardType::Artifact), Rest::BottomRandom)?;
    Ok(())
}

/// "You may put an artifact card from your hand onto the battlefield. If it's an Equipment, attach it to The Invincible Iron
/// Man." (CR 701.3a, 301.5): the card put onto the battlefield, if it is an Equipment still there, becomes attached to this
/// creature if it is on the battlefield (a new object, CR 400.7).
fn put_an_artifact_from_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(put) = hand::may_put_onto_battlefield(g, item.controller, item.object, |c| c.is(CardType::Artifact))? else { return Ok(()) };
    if g.is_on_battlefield(put) && g.is_on_battlefield(item.object) && g.chars(put).has_subtype(Subtype::Equipment) {
        g.attach(put, item.object);
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        put_an_artifact_from_hand,
        "At the beginning of combat on your turn, you may put an artifact card from your hand onto the battlefield. If it's an Equipment, attach it to The Invincible Iron Man.",
    )],
    ..oracle::THE_INVINCIBLE_IRON_MAN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}, {T}"),
            dig_for_an_artifact,
            "{1}, {T}: Look at the top four cards of your library. You may reveal an artifact card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
        ),
        ability(cost("{4}{U}{R}"), effects::transform_self, "{4}{U}{R}: Transform Tony Stark. Activate only as a sorcery.").sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::TONY_STARK
};
