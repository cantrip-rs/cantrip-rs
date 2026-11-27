use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// How the chosen card is played (CR 305.1, 601.2): as a land, by the face its way names (`Game::land_ways_during_resolution_by`),
/// or cast, by the way its `CastWay` names.
#[derive(Clone, Copy)]
enum Play {
    Land(u8),
    Cast(u8),
}

/// "Look at target opponent's hand and choose a card from it. You control that player until Word of Command finishes
/// resolving. The player plays that card if able. While doing so, the player can activate mana abilities only if they're
/// from lands that player controls and only if mana they produce is spent to activate other mana abilities of lands the
/// player controls and/or to play that card. If the chosen card is cast as a spell, you control the player while that spell
/// is resolving." (CR 723.2, 723.5, 723.7, 608.2g, 305.1)
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    g.look_at(item.controller, &hand);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Play, hand, select::CARD)?;
    g.control_until_resolved(item, player);
    let Some(&card) = chosen.first() else { return Ok(()) };
    // To play it is to play it as a land - only in that player's turn, with a land play left (CR 305.2-3; the rulings) - or to
    // cast it (CR 701.18b), by a way of casting it its player could pay for (CR 608.2g; "if there is a non-mana way to cast
    // a spell, ... you may require that way to be used").
    // "While doing so ...": only lands' mana abilities, each only if some of its mana is spent on the card; the rest is left
    // as ordinary mana, and mana already floating may pay (owner ruling 2026-11-27, `CastTerms::land_mana_only`).
    let terms = CastTerms { caster: Some(player), if_able: true, land_mana_only: true, ..CastTerms::default() };
    let face = |g: &dyn Game, way: u8| {
        let printed = g.state().obj(card).card;
        match (way, g.card(printed).back) {
            (WAY_BACK_FACE, Some(back)) => g.card_id(back),
            _ => printed,
        }
    };
    let mut plays: Vec<(Label, Play)> = g.land_ways_during_resolution_by(card, player).into_iter().map(|way| (Label::Face(face(g, way)), Play::Land(way))).collect();
    for way in g.cast_ways_during_resolution_as(card, terms) {
        let label = if way.way == WAY_FUSED { Label::Fused(way.face) } else { Label::Face(way.face) };
        plays.push((label, Play::Cast(way.way)));
    }
    // The player's choice, and so the controller's (CR 723.5): which face, where the card can be played more than one way.
    let play = match plays.len() {
        0 => return Ok(()),
        1 => plays[0].1,
        _ => plays[choose::one_of(g, player, card, plays.iter().map(|p| p.0.clone()).collect())? as usize].1,
    };
    match play {
        Play::Land(way) => {
            g.play_land_during_resolution_by(card, player, way)?;
        }
        Play::Cast(way) => {
            if g.cast_during_resolution_as(card, CastTerms { way: Some(way), ..terms })?
                && let Some(spell) = g.state().stack.last().map(|i| i.object)
            {
                g.control_while_spell_resolves(item, player, spell);
            }
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], effect),
    ..oracle::WORD_OF_COMMAND
};
