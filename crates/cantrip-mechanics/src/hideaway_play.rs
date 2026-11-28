//! Playing the card a hideaway permanent exiled (CR 702.75a, 608.2g).

use crate::play_during_resolution;
use cantrip_api::*;

/// "You may play the exiled card without paying its mana cost" as the ability `item` resolves (CR 118.9, 305.1, 608.2g): the
/// card found by the exile link off the ability's source as it was, by its text (CR 607.2a). A land is played then or not at all; a spell is cast with its timing
/// waived. To play a modal double-faced card is to cast its front or play its land back face, the player's choice (CR 712.11b,
/// 712.12; `play_during_resolution::ways`). The controller is asked first, and nothing is asked when the card could not be
/// played at all. Windbrisk Heights, Shelldock Isle, Mosswort Bridge, Collector's Cage and Clive's Hideaway print it.
pub fn play_exiled_card(g: &mut dyn Game, item: &StackItem, you: PlayerId) -> Result<(), Suspend> {
    let Some(&card) = g.exiled_with(g.linked(item)).first() else { return Ok(()) };
    if g.land_ways_during_resolution_by(card, you).is_empty() {
        crate::effects::may_cast_without_paying(g, you, card)?;
        return Ok(());
    }
    let terms = CastTerms { instead: Some(ManaCost::ZERO), caster: Some(you), ..CastTerms::default() };
    // A cast that can't be made or that the player gives up leaves them choosing again among the ways not yet tried, the
    // land face among them (CR 733.1; `play_during_resolution::may_play`).
    play_during_resolution::may_play(g, you, card, terms, true, Purpose::Ask(&PLAY_EXILED_LAND))?;
    Ok(())
}

/// What playing an exiled land asks: a land played this way is the turn's land play (the hideaway rulings).
const PLAY_EXILED_LAND: Question = Question::new(
    "PlayLandDuringResolution",
    "Play {source}?",
    "As a land, it uses your land play this turn. If you don't, it stays in exile",
    "Play it",
    "Don't play",
);
