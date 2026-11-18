use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, find_to_battlefield};
use cantrip_mechanics::select::LAND_WITH_A_BASIC_LAND_TYPE_CARD;
use cantrip_mechanics::{channel, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("SearchLibrary", "search your library for a land card with a basic land type?", "It goes onto the battlefield, then you shuffle", "Search", "Don't search");

/// "Target artifact, enchantment, or nonbasic land an opponent controls" (CR 115.1).
fn opponents_target(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id)
        && q.controller(id) != asker.controller && {
            let c = q.chars(id);
            c.is(CardType::Artifact) || c.is(CardType::Enchantment) || (c.is(CardType::Land) && !c.supertypes.basic)
        })
}

/// Channel: destroy the target. Its controller (as it last was, CR 608.2h) may search their library for a land card with
/// a basic land type, put it onto the battlefield, then shuffle. The "may" is asked of that player, and declining skips
/// the search and the shuffle. Nothing happens at all if the target is no longer legal.
fn destroy_and_search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let player = g.controller(target);
    g.destroy(target);
    if !ask_may_search(g, player, item.object, &ASK)? {
        return Ok(());
    }
    find_to_battlefield(g, player, item.object, LAND_WITH_A_BASIC_LAND_TYPE_CARD, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        ability(cost("{1}{G}").and(DISCARD_THIS), destroy_and_search, "Channel \u{2014} {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That player may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle. This ability costs {1} less to activate for each legendary creature you control.")
            .targets(&[target(opponents_target)])
            .from(ZoneKind::Hand)
            .cost_change(channel::cheaper_per_legendary_creature),
    ],
    ..oracle::BOSEIJU_WHO_ENDURES
};
