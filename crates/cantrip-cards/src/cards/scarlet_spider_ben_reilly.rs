use cantrip_api::*;
use cantrip_mechanics::web_slinging;
use cantrip_oracle as oracle;

/// The creature returned to its owner's hand to pay for casting `spell` by web-slinging (CR 702.188a, 601.2h), as it last
/// existed on the battlefield: the last permanent the history shows leaving the battlefield for a hand before the spell
/// became cast (CR 601.2i). Its costs are paid after every mana ability activated while casting it has resolved (CR
/// 601.2g-h), and web-slinging's return is the first of them paid, so a permanent a mana ability bounced is earlier than it;
/// the spell has no other cost that returns a permanent. As Kazuul's Fury reads its sacrifice.
fn returned_creature(q: &dyn Query, spell: ObjectId) -> Option<ObjectId> {
    let history = q.history();
    let cast = history.all().iter().rev().find(|r| matches!(r.event, Event::Cast(c) if c.spell == spell))?.serial;
    let returned = history
        .select(EventQuery::kind(EventMask::ZONE_CHANGE).from(ZoneKind::Battlefield).to(ZoneKind::Hand))
        .rev()
        .find(|r| r.serial < cast)?;
    let Event::ZoneChange { old, .. } = returned.event else { return None };
    Some(old)
}

/// Sensational Save: "If Scarlet Spider was cast using web-slinging, he enters with X +1/+1 counters on him, where X is
/// the mana value of the returned creature." (CR 614.1c, 702.188a) The mana value is the creature's as it last existed on
/// the battlefield, an {X} in its cost counting 0 (ruling 2025-09-19, CR 202.3b).
fn sensational_save(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if web_slinging::was_cast_using_web_slinging(g, entering.object)
        && let Some(creature) = returned_creature(g, entering.object)
    {
        entering.counters[CounterKind::PlusOne as usize] += i64::from(g.mana_value(creature).min(i16::MAX as u32) as i16);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[web_slinging::way(mana("{R}{G}"), "Web-slinging {R}{G}")],
    enters: Some(sensational_save),
    ..oracle::SCARLET_SPIDER_BEN_REILLY
};
