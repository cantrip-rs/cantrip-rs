use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::{tokens, top_of_library, triggers};
use cantrip_oracle as oracle;

const TO_GRAVEYARD: Question = Question::new(
    "PutRevealedIntoGraveyard",
    "put the revealed card into your graveyard?",
    "It isn't a creature card with mana value 3 or less. If you don't, it stays on top of your library",
    "Put it into your graveyard",
    "Leave it",
);

/// "Whenever Kellan attacks, reveal the top card of your library. If it's a creature card with mana value 3 or less, put it into
/// your hand. Otherwise, you may put it into your graveyard." (CR 701.20a, 400.3, 608.2h): the card is revealed whatever it is,
/// an empty library reveals nothing, and a card that doesn't qualify stays on top unless its controller sends it to the graveyard.
fn reveal_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let Some(&top) = top_of_library(g, player, 1).first() else { return Ok(()) };
    g.reveal(player, &[top]);
    if g.chars(top).is(CardType::Creature) && g.mana_value(top) <= 3 {
        g.return_to_hand_revealed(&[top]);
    } else if ask_may(g, player, item.object, Purpose::Ask(&TO_GRAVEYARD))? {
        g.put_into_graveyard(&[top]);
    }
    Ok(())
}

/// "Create X Map tokens, where X is one plus the number of opponents who control an artifact." (CR 111.1, 111.10s): the artifacts
/// are counted as the spell resolves, through the layers, and an opponent counts once however many artifacts they control. The
/// Maps are created together (CR 101.4), so what watches one or more tokens enter sees them once.
fn create_x_map_tokens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let with_artifact = opponents
        .into_iter()
        .filter(|&opponent| g.state().battlefield().iter().any(|&id| g.controller(id) == opponent && g.chars(id).is(CardType::Artifact)))
        .count();
    tokens::create_by_rule(g, item.controller, &tokens::MAP, with_artifact as u32 + 1, false)?;
    Ok(())
}

/// The Adventure (CR 715): Journey On.
pub static JOURNEY_ON: CardDef = CardDef {
    spell: spell(&[], create_x_map_tokens),
    ..oracle::JOURNEY_ON
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        reveal_top_card,
        "Whenever Kellan attacks, reveal the top card of your library. If it's a creature card with mana value 3 or less, put it into your hand. Otherwise, you may put it into your graveyard.",
    )],
    adventure: Some(&JOURNEY_ON),
    ..oracle::KELLAN_DARING_TRAVELER
};

pub static EXTRA: &[&CardDef] = &[&JOURNEY_ON, &tokens::MAP];
