use cantrip_api::*;
use cantrip_mechanics::attach::equip;
use cantrip_mechanics::choose;
use cantrip_mechanics::library::rest_to_bottom_in_random_order;
use cantrip_mechanics::top_of_library;
use cantrip_mechanics::triggers::EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT;
use cantrip_oracle as oracle;

/// "Look at that many cards from the top of your library. You may exile a nonland card from among them. Put the rest on the
/// bottom of your library in a random order. You may cast the exiled card without paying its mana cost" (CR 401.4, 608.2c,
/// 608.2g, 118.9). "That many" is the damage the trigger carried from the event (`matched_damage_amount`), so it does not change
/// if the creature or the Equipment changes before the ability resolves; a library with fewer cards shows what it has (CR 609.3).
/// A land card is no choice for the exile, and with no nonland card among them the cards are still looked at and all go to the
/// bottom. Only a card that really arrived in exile (a replacement effect could have sent it elsewhere) is the exiled card: it
/// may be cast for nothing, and a land is not offered because it has no cast.
fn look_exile_a_nonland_and_may_cast_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, item.matched_damage_amount() as usize);
    if looked.is_empty() {
        return Ok(());
    }
    g.look_at(you, &looked);
    let nonland: Vec<ObjectId> = looked.iter().copied().filter(|&c| !g.chars(c).is(CardType::Land)).collect();
    let most = nonland.len().min(1) as u32;
    let ask = RequestKind::DistributeCards {
        source: item.object,
        cards: looked.clone(),
        destinations: vec![
            Destination { kind: DestinationKind::Exile, min: 0, max: most, eligible: nonland },
            Destination { kind: DestinationKind::LibraryBottom, min: 0, max: u32::MAX, eligible: looked.clone() },
        ],
        may_shuffle: false,
    };
    let Answer::Placements { placements, .. } = g.ask(you, ask)? else { return Ok(()) };
    let chosen = placements.first().and_then(|p| p.first().copied());
    let exiled = chosen.map(|card| g.exile(card)).filter(|&card| g.obj(card).zone.is(ZoneKind::Exile));
    rest_to_bottom_in_random_order(g, you, &looked);
    if let Some(card) = exiled {
        choose::may_cast_free_card(g, you, item.object, card, u32::MAX, Purpose::CastFree)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
            look_exile_a_nonland_and_may_cast_it,
            "Whenever equipped creature deals combat damage to a player, look at that many cards from the top of your library. You may exile a nonland card from among them. Put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost.",
        )
        .damage_amount(),
        equip(mana("{2}{U}"), "Equip {2}{U}"),
    ],
    ..oracle::THE_KEY_TO_THE_VAULT
};
