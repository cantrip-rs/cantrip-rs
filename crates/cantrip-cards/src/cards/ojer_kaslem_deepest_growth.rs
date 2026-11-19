use cantrip_api::*;
use cantrip_mechanics::library::rest_to_bottom_in_random_order;
use cantrip_mechanics::triggers::{DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT, THIS_DIES};
use cantrip_mechanics::{choose, effects, select, top_of_library};
use cantrip_oracle as oracle;

/// "Reveal that many cards from the top of your library. You may put a creature card and/or a land card from among them onto the battlefield. Put the rest
/// on the bottom in a random order" (CR 701.20, 608.2d, 110.2a, 401.4): the amount is the combat damage dealt, and a library with fewer cards reveals what it
/// has (CR 609.3). Up to one creature card and up to one land card are chosen with every revealed card in view, before either moves, and enter together; a
/// card that is both can be the creature or the land but not both, so the land is chosen from the rest. Whatever is left, a chosen card that could not enter
/// included, goes on the bottom.
fn reveal_and_put_a_creature_and_or_a_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let revealed = top_of_library(g, you, item.data[0] as usize);
    if revealed.is_empty() {
        return Ok(());
    }
    g.reveal(you, &revealed);
    let mut chosen = choose::select(g, you, item.object, Purpose::PutOntoBattlefield, revealed.clone(), select::creature_cards(select::up_to(1)))?;
    let rest: Vec<ObjectId> = revealed.iter().copied().filter(|card| !chosen.contains(card)).collect();
    chosen.extend(choose::select(g, you, item.object, Purpose::PutOntoBattlefield, rest, select::land_cards(select::up_to(1)))?);
    let entering: Vec<(ObjectId, PlayerId)> = chosen.into_iter().map(|card| (card, you)).collect();
    g.put_all_onto_battlefield(&entering)?;
    rest_to_bottom_in_random_order(g, you, &revealed);
    Ok(())
}

/// "Activate only if you control ten or more permanents" (CR 602.5b): permanents under the activator's control as the ability is activated, itself included.
fn you_control_ten_or_more_permanents(q: &dyn Query, asker: &Asker) -> bool {
    q.state().battlefield().iter().filter(|&&id| q.controller(id) == asker.controller).count() >= 10
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        ability(cost("{2}{G}, {T}"), effects::transform_self, "{2}{G}, {T}: Transform this land. Activate only if you control ten or more permanents and only as a sorcery.")
            .sorcery_speed()
            .condition(you_control_ten_or_more_permanents),
    ],
    ..oracle::TEMPLE_OF_CULTIVATION
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
            reveal_and_put_a_creature_and_or_a_land,
            "Whenever Ojer Kaslem deals combat damage to a player, reveal that many cards from the top of your library. You may put a creature card and/or a land card from among them onto the battlefield. Put the rest on the bottom in a random order.",
        ),
        triggered(THIS_DIES, effects::return_it_tapped_and_transformed, "When Ojer Kaslem dies, return it to the battlefield tapped and transformed under its owner's control."),
    ],
    back: Some(&BACK),
    ..oracle::OJER_KASLEM_DEEPEST_GROWTH
};
