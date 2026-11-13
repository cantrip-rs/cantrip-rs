use cantrip_api::*;
use cantrip_mechanics::{effects, hand, triggers};
use cantrip_oracle as oracle;

/// "Whenever another Klingon or Warrior you control dies" (CR 603.2).
fn when_another_klingon_warrior_control_dies(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { old, from, to, .. } = event else { return None };
    let id = *old;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Graveyard) && from.is(ZoneKind::Battlefield)
        && (!(asker.source == id)
            && ((q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Klingon))
                || q
                    .chars(id)
                    .has_subtype(q.words(asker.source).subtype(Subtype::Warrior)))
                && asker.controller == q.controller(id)))
    {
        return Fired::about(subject);
    }
    None
}

/// "When Curzon Dax dies, you may put a creature card with mana value 3 or less from your hand onto the battlefield." (CR 608.2d).
fn put_creature_card_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::may_put_onto_battlefield_judged(g, item.controller, item.object, |g, c| g.chars(c).is(CardType::Creature) && g.mana_value(c) <= 3)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask::ZONE_CHANGE,
                when_another_klingon_warrior_control_dies,
            ),
            effects::damage_each_opponent_n::<1>,
            "Whenever another Klingon or Warrior you control dies, Curzon Dax deals 1 damage to each opponent.",
        ),
        triggered(
            triggers::THIS_DIES,
            put_creature_card_mana_value,
            "When Curzon Dax dies, you may put a creature card with mana value 3 or less from your hand onto the battlefield.",
        ),
    ],
    ..oracle::CURZON_DAX_BOUND_BY_BLOOD
};
