use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Spells can't be countered." (CR 604.1).
fn spells_cant_be_countered_affects(
    q: &dyn LayerQuery,
    _: ObjectId,
    affected: ObjectId,
    _: &Chars,
) -> bool {
    q.state().obj(affected).became.is_none()
        && q.state().obj(affected).zone.kind() == ZoneKind::Stack
}

/// "Spells can't be countered." (CR 604.1).
fn spells_cant_be_countered_apply_0(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::CantBeCountered);
}

/// "Spells can't be countered. Each instant and sorcery card in your graveyard has flashback. The flashback cost is equal to that card's mana cost." (CR 608.2).
fn spells_cant_be_countered_instant_cost(
    q: &dyn Query,
    asker: &Asker,
    card: ObjectId,
    player: PlayerId,
) -> Option<Cost> {
    (player == asker.controller
        && (q.chars(card).is(CardType::Instant) || q.chars(card).is(CardType::Sorcery))
        && q.card(q.chars(card).card).has_cost)
        .then(|| Cost::mana(q.card(q.chars(card).card).cost))
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
                Layer::Ability,
                spells_cant_be_countered_affects,
                spells_cant_be_countered_apply_0,
                "Spells can't be countered.",
            )
            .everywhere(),
    ],
    granted_ways: &[
        GrantedWay {
            text: "Flashback",
            flashback: true,
            cost: spells_cant_be_countered_instant_cost,
            ..GrantedWay::CAST
        },
    ],
    ..oracle::LIER_DISCIPLE_OF_THE_DROWNED
};
