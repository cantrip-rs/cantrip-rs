use cantrip_api::*;
use cantrip_mechanics::surveil;
use cantrip_oracle as oracle;

/// "Surveil 3. (Look at the top three cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)" (CR 608.2).
fn surveil_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    surveil(g, item.controller, item.object, 3)?;
    Ok(())
}

/// "Surveil 3. (Look at the top three cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)" (CR 608.2).
fn surveil_3_require(q: &dyn Query, asker: &Asker) -> bool {
    q.chars(asker.source).is(CardType::Instant)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], surveil_3),
    ways: &[
        way(cost("{1}{U}"), ZoneKind::Graveyard, "Flashback")
            .only_from()
            .exile_after()
            .condition(surveil_3_require),
    ],
    ..oracle::OTHERWORLDLY_GAZE
};
