use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

/// "Look at the top X cards of your library, where X is the amount of mana spent to cast this spell. Put two of them into your hand and the rest on the bottom of your library in a random order." (CR 608.2).
fn look_top_x_cards_library(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let depth_0 = {
        let q: &dyn Query = g;
        ({
            let spent = q.obj(item.object).spent;
            [Mana::W, Mana::U, Mana::B, Mana::R, Mana::G, Mana::C]
                .into_iter()
                .map(|mana| i64::from(spent.get(mana)))
                .sum::<i64>()
        }) as usize
    };
    library::look_at_top(
        g,
        item.controller,
        item.object,
        depth_0,
        (2, 2),
        false,
        |_| true,
        library::Rest::BottomRandom,
    )?;
    Ok(())
}

/// "Look at the top X cards of your library, where X is the amount of mana spent to cast this spell. Put two of them into your hand and the rest on the bottom of your library in a random order." (CR 608.2).
fn look_top_x_cards_library_require(q: &dyn Query, asker: &Asker) -> bool {
    q.chars(asker.source).is(CardType::Instant)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], look_top_x_cards_library),
    ways: &[
        way(cost("{5}{U}{U}"), ZoneKind::Graveyard, "Flashback")
            .only_from()
            .exile_after()
            .condition(look_top_x_cards_library_require),
    ],
    ..oracle::MEMORY_DELUGE
};
