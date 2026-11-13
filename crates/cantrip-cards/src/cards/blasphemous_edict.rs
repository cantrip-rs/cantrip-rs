use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "If there are thirteen or more creatures on the battlefield" (CR 118.9, 601.2b): counted, under every player's control, when
/// the spell is cast, as the alternative cost is chosen.
fn thirteen_or_more_creatures(q: &dyn Query, _: &Asker) -> bool {
    q.state().battlefield().iter().filter(|&&id| q.chars(id).is(CardType::Creature)).count() >= 13
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::each_player_sacrifices_creatures::<13>),
    ways: &[way(cost("{B}"), ZoneKind::Hand, "Pay {B}").condition(thirteen_or_more_creatures)],
    ..oracle::BLASPHEMOUS_EDICT
};
