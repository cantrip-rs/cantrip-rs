use cantrip_api::*;
use cantrip_mechanics::{effects, lands, statics};
use cantrip_oracle as oracle;

/// "As long as Captain America has a shield counter on him" (CR 122.1c, 604.1): asked live as the layers are worked out.
fn has_a_shield_counter(q: &dyn LayerQuery, source: ObjectId) -> bool {
    q.state().obj(source).counters(CounterKind::Shield) > 0
}

/// "You ... have hexproof" (CR 702.11c) while he has a shield counter: his controller.
fn you_while_shielded(q: &dyn LayerQuery, source: ObjectId, player: PlayerId) -> bool {
    has_a_shield_counter(q, source) && q.state().obj(source).controller == player
}

/// "Other Heroes you control have hexproof" (CR 702.11b, 613.1f, layer 6) while he has a shield counter: every other permanent
/// his controller controls that is a Hero as the earlier layers made it, the type read through the words of this card's text
/// (CR 612).
fn other_heroes_while_shielded(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    has_a_shield_counter(q, source)
        && statics::other_of_subtype_you_control::<{ lands::sub(Subtype::Hero) }>(q, source, affected, so_far)
}

/// "Captain America enters with a shield counter on him" (CR 614.1c), whose effects the core has (CR 122.1c); "you ... have
/// hexproof" is a player keyword ability of the permanent, and "other Heroes you control have hexproof" a layer-6 static
/// ability.
pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_a_shield_counter),
    player_keywords: &[PlayerKeywordRule { keyword: Keyword::Hexproof, applies: you_while_shielded }],
    statics: &[static_ability(
        Layer::Ability,
        other_heroes_while_shielded,
        statics::have_hexproof,
        "As long as Captain America has a shield counter on him, you and other Heroes you control have hexproof.",
    )],
    ..oracle::CAPTAIN_AMERICA_SUPER_SOLDIER
};
