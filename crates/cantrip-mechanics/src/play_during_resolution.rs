//! Playing a card as an effect resolves (CR 305.1, 601.2, 608.2g): every way it may be played, as a land or cast.

use cantrip_api::*;

/// One way a card may be played as an effect resolves (CR 305.1, 601.2): as a land, by the way
/// `Game::land_ways_during_resolution_by` names (the card as itself, or a modal double-faced card's land back face, CR
/// 712.12), or cast, by the way its `CastWay` names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Play {
    Land(u8),
    Cast(u8),
}

/// Each way `player` could play `card` now as an effect resolves (CR 305.1, 608.2g, 712.12), each labelled with the face it
/// plays (CR 709.3, 712.11b, 712.12): every face they could play as a land - their own turn, a land play left (CR 305.2-3) -
/// then every way it could be cast by `terms` (`Game::cast_ways_during_resolution_as`, judged for `terms.caster`). A modal
/// double-faced card with a land back face is both: its front cast and its back played (CR 712.11b, 712.12). Word of Command and the hideaway
/// lands' "play the exiled card" ask it.
pub fn ways(g: &mut dyn Game, card: ObjectId, player: PlayerId, terms: CastTerms) -> Vec<(Label, Play)> {
    let printed = g.state().obj(card).card;
    let back = g.card(printed).back.map(|back| g.card_id(back));
    let mut plays: Vec<(Label, Play)> = g
        .land_ways_during_resolution_by(card, player)
        .into_iter()
        .map(|way| (Label::Face(if way == WAY_BACK_FACE { back.unwrap_or(printed) } else { printed }), Play::Land(way)))
        .collect();
    plays.extend(cast_ways(g, card, terms));
    plays
}

/// Each way `card` could be cast by `terms` now as an effect resolves (`Game::cast_ways_during_resolution_as`), labelled with
/// the face it casts (CR 709.3, 712.11b), a fused cast as both halves (CR 702.102a).
fn cast_ways(g: &mut dyn Game, card: ObjectId, terms: CastTerms) -> Vec<(Label, Play)> {
    g.cast_ways_during_resolution_as(card, terms)
        .into_iter()
        .map(|way| (if way.way == WAY_FUSED { Label::Fused(way.face) } else { Label::Face(way.face) }, Play::Cast(way.way)))
        .collect()
}

/// "[`player`] may play [`card`]" - or, without `lands`, "may cast" it - by `terms` as an effect resolves (CR 305.1, 601.2,
/// 608.2g): asked with `purpose` whether to, then which way where there is more than one (`choose`). A land face is played
/// then or not at all. A cast that can't be made or that the player gives up is reversed to the moment before it was begun
/// (CR 733.1), so they are asked again, offered the ways not yet tried: nothing has changed since, so a way reversed as
/// illegal would be again, and giving a cast up promises no second try of it. Each pass offers one way fewer, so the
/// questions end with the ways, whatever the player answers. Whether the card was played. The hideaway lands' "play the
/// exiled card" and `effects::may_cast_without_paying` ask it.
pub fn may_play(g: &mut dyn Game, player: PlayerId, card: ObjectId, terms: CastTerms, lands: bool, purpose: Purpose) -> Result<bool, Suspend> {
    let mut tried: Vec<Play> = Vec::new();
    loop {
        let mut plays = if lands { ways(g, card, player, terms) } else { cast_ways(g, card, terms) };
        plays.retain(|(_, play)| !tried.contains(play));
        if plays.is_empty() || !crate::effects::ask_may(g, player, card, purpose)? {
            return Ok(false);
        }
        let Some(play) = choose(g, player, card, &plays)? else { return Ok(false) };
        match play {
            Play::Land(way) => return Ok(g.play_land_during_resolution_by(card, player, way)?.is_some()),
            Play::Cast(way) => {
                if g.cast_during_resolution_as(card, CastTerms { way: Some(way), ..terms })? {
                    return Ok(true);
                }
                tried.push(play);
            }
        }
    }
}

/// The play `player` chooses among `plays` (`ways`), the face they play the card as (CR 712.12, 709.3): asked only when there
/// is more than one. None when there is none.
pub fn choose(g: &mut dyn Game, player: PlayerId, card: ObjectId, plays: &[(Label, Play)]) -> Result<Option<Play>, Suspend> {
    Ok(match plays.len() {
        0 => None,
        1 => Some(plays[0].1),
        _ => Some(plays[crate::choose::one_of(g, player, card, plays.iter().map(|p| p.0.clone()).collect())? as usize].1),
    })
}
