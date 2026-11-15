use cantrip_api::*;
use cantrip_mechanics::{effects, replacements};
use cantrip_oracle as oracle;

/// "During your turn, you may cast instant and sorcery spells from your graveyard by paying 1 life in addition to their other
/// costs" (CR 604, 601.2b, 601.2f, 118.8): a permission (`GrantedWay::permission`), so an alternative cost may be paid instead.
/// The spell is cast for its mana cost with 1 life owed on top (`GrantedWay::additional`); only its controller's own cards, from
/// their own graveyard, on their own turn; a card with no mana cost can't be cast for it (CR 118.6). The life is payable only with at least 1 (CR 119.4).
fn instant_or_sorcery_from_your_graveyard(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> Option<Cost> {
    if player != asker.controller || q.state().turn.active != player || q.obj(card).owner != player {
        return None;
    }
    let c = q.chars(card);
    if !(c.is(CardType::Instant) || c.is(CardType::Sorcery)) {
        return None;
    }
    let def = q.card(c.card);
    def.has_cost.then(|| Cost::mana(def.cost))
}

pub static CARD: CardDef = CardDef {
    granted_ways: &[GrantedWay {
        text: "During your turn, you may cast instant and sorcery spells from your graveyard by paying 1 life in addition to their other costs.",
        cost: instant_or_sorcery_from_your_graveyard,
        permission: true,
        additional: life(1),
        ..GrantedWay::CAST
    }],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        replacements::card_or_token_to_your_graveyard,
        replacements::exile_instead_apply,
        "If a card or token would be put into your graveyard from anywhere, exile it instead.",
    )],
    abilities: &[ability(cost("{1}{R}"), effects::sacrifice_self, "{1}{R}: Sacrifice this enchantment.")],
    ..oracle::FESTIVAL_OF_EMBERS
};
