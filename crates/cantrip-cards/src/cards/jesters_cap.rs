use cantrip_api::*;
use cantrip_mechanics::{library, select, targets};
use cantrip_oracle as oracle;

/// "{2}, {T}, Sacrifice this artifact: Search target player's library for three cards and exile them. Then that player shuffles." (CR 608.2, 701.13a, 701.24a).
fn search_target_players_library_three(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(owner) = targets::player(g, item, 0) else { return Ok(()) };
    let found = library::search(
        g,
        item.controller,
        owner,
        item.object,
        SearchDestination::Exile,
        select::cards(select::as_many_as_can(3)),
    )?;
    g.exile_all(&found);
    g.shuffle(owner);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(SACRIFICE_THIS).and(TAP),
                search_target_players_library_three,
                "{2}, {T}, Sacrifice this artifact: Search target player's library for three cards and exile them. Then that player shuffles.",
            )
            .targets(&[targets::PLAYER]),
    ],
    ..oracle::JESTERS_CAP
};
