use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "{W}, {T}, Exert Basri: Create a 1/1 white Cat creature token with lifelink."
fn make_lifelink_cat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::CAT_LIFELINK, 1)?;
    Ok(())
}

/// "Cats you control gain hexproof and indestructible until end of turn." Every Cat its controller has as the ability resolves,
/// the token and the printed ones alike (CR 205.3m, 611.2c, 702.11, 702.12).
fn cats_gain_hexproof_and_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cat = g.words(item.object).subtype(Subtype::Cat);
    effects::creatures_you_control_where_gain(g, item, &[Keyword::Hexproof, Keyword::Indestructible], |g, id| g.chars(id).has_subtype(cat));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{W}, {T}").and(EXERT), make_lifelink_cat, "{W}, {T}, Exert Basri: Create a 1/1 white Cat creature token with lifelink. (An exerted creature won't untap during your next untap step.)"),
        triggers::cycling(mana("{2}{W}"), "Cycling {2}{W} ({2}{W}, Discard this card: Draw a card.)"),
        triggered(triggers::YOU_CYCLE_THIS_CARD, cats_gain_hexproof_and_indestructible, "When you cycle this card, Cats you control gain hexproof and indestructible until end of turn."),
    ],
    ..oracle::BASRI_TOMORROWS_CHAMPION
};

pub static EXTRA: &[&CardDef] = &[&tokens::CAT_LIFELINK];
