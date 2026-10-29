use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Another creature or token" (CR 701.21a): a permanent other than this one that is a creature, or a token of any type. The sacrifice
/// is a cost, so it is chosen from the permanents its controller controls.
fn another_creature_or_token(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    id != asker.source && (q.chars(id).is(CardType::Creature) || q.obj(id).token)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}").and(sacrifice(1, another_creature_or_token)),
            |g, item| {
                g.draw(item.controller, 1);
                Ok(())
            },
            "{2}, Sacrifice another creature or token: Draw a card. Activate only as a sorcery.",
        )
        .sorcery_speed(),
        ability(
            cost("{2}, {T}").and(SACRIFICE_THIS),
            |g, item| effects::gain_life(g, item, 3),
            "{2}, {T}, Sacrifice this creature: You gain 3 life.",
        ),
    ],
    ..oracle::ICE_CREAM_KITTY
};
