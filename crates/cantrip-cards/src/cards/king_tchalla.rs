use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_oracle as oracle;

/// "Whenever a player draws their second card each turn" (CR 120.3, 603.2): any player's, you included.
fn a_players_second_draw(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    cantrip_mechanics::history::second_card_drawn_this_turn(q, event).and(Fired::PLAIN)
}


/// "Prevent all damage that would be dealt to Black Panther" (CR 615.1, 604.1): a static ability's prevention effect,
/// of all damage to this permanent from any source, combat or not, while it is on the battlefield.
fn dealt_to_black_panther(_: &dyn Query, asker: &Asker, damage: &DamageEvent) -> bool {
    damage.target == Target::Object(asker.source)
}

pub static BACK: CardDef = CardDef {
    preventions: &[PreventionDef { hint: "prevents all damage that would be dealt to Black Panther", applies: dealt_to_black_panther, amount: PreventionAmount::All }],
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER,
        |g, item| effects::draw(g, item, 1),
        "Whenever Black Panther deals combat damage to a player, draw a card.",
    )],
    ..oracle::BLACK_PANTHER_HOPE_ENDURING
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::DREW, a_players_second_draw),
            |g, item| effects::draw(g, item, 1),
            "Whenever a player draws their second card each turn, you draw a card.",
        ),
        ability(cost("{4}{W}{U}"), effects::transform_self, "{4}{W}{U}: Transform King T'Challa. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::KING_TCHALLA
};
