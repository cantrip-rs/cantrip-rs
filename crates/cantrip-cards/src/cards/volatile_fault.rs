use cantrip_api::*;
use cantrip_mechanics::lands::destroy_target_its_controller_may_fetch_basic;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target nonbasic land an opponent controls. That player may search their library for a basic land card, put it
/// onto the battlefield, then shuffle. You create a Treasure token." (CR 701.8, 701.23, 111.10a.) With its only target
/// illegal the ability does not resolve at all (CR 608.2b), so no Treasure; otherwise the search is offered and the
/// Treasure made even if the land was not destroyed (ruling 2023-11-10).
fn destroy_search_and_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target_its_controller_may_fetch_basic(g, item, 0, false)?;
    g.end_sentence();
    tokens::create_treasure(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{1}").and(TAP).and(SACRIFICE_THIS), destroy_search_and_treasure, "{1}, {T}, Sacrifice this land: Destroy target nonbasic land an opponent controls. That player may search their library for a basic land card, put it onto the battlefield, then shuffle. You create a Treasure token.")
            .targets(&[targets::NONBASIC_LAND_AN_OPPONENT_CONTROLS]),
    ],
    ..oracle::VOLATILE_FAULT
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
