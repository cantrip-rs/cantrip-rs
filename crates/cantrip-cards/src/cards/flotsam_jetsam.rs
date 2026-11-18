use cantrip_api::*;
use cantrip_mechanics::{choose, effects, tokens};
use cantrip_oracle as oracle;

/// Flotsam: "Mill three cards. Investigate."
fn flotsam(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::you_mill::<3>(g, item)?;
    tokens::investigate(g, item.controller)
}

/// Jetsam: "Each opponent mills three cards, then you may cast a spell from each opponent's graveyard without paying its mana
/// cost. If a spell cast this way would be put into a graveyard, exile it instead." One spell at most from each opponent's
/// graveyard, offered opponent by opponent in turn order; nothing is asked of an opponent whose graveyard holds nothing castable.
fn jetsam(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::each_opponent_mills::<3>(g, item)?;
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    for opponent in opponents {
        let graveyard = g.state().zone(Zone::graveyard(opponent)).to_vec();
        choose::cast_up_to_among(
            g,
            item.controller,
            item.object,
            Purpose::CastFree,
            &graveyard,
            1,
            CastTerms { instead: Some(ManaCost::ZERO), exile_instead: ExileInstead::Spell, ..CastTerms::default() },
            |_, _| true,
        )?;
    }
    Ok(())
}

pub static FLOTSAM: CardDef = CardDef { spell: spell(&[], flotsam), ..oracle::FLOTSAM };

pub static JETSAM: CardDef = CardDef { spell: spell(&[], jetsam), ..oracle::JETSAM };

pub static CARD: CardDef = CardDef { split: Some((&FLOTSAM, &JETSAM)), ..oracle::FLOTSAM_JETSAM };

pub static EXTRA: &[&CardDef] = &[&FLOTSAM, &JETSAM, &tokens::CLUE];
