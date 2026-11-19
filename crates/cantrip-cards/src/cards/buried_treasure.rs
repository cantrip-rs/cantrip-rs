use cantrip_api::*;
use cantrip_mechanics::discover;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP.and(SACRIFICE_THIS), add_any_color(1), "{T}, Sacrifice this artifact: Add one mana of any color."),
        ability(
            cost("{5}").and(EXILE_THIS),
            |g, item| {
                discover::discover(g, item.controller, 5)?;
                Ok(())
            },
            "{5}, Exile this card from your graveyard: Discover 5. Activate only as a sorcery.",
        )
        .from(ZoneKind::Graveyard)
        .sorcery_speed(),
    ],
    ..oracle::BURIED_TREASURE
};
