use cantrip_api::*;
use cantrip_mechanics::{impending, targets};
use cantrip_oracle as oracle;

/// "Whenever this permanent enters or attacks" (CR 603.2).
fn when_permanent_enters_attacks(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    if let Event::ZoneChange { new, to, .. } = event {
        let id = *new;
        let subject = Target::Object(id);
        if to.is(ZoneKind::Battlefield) && asker.source == id {
            return Fired::about(subject);
        }
    }
    let Event::Attacks { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    if asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "Whenever this permanent enters or attacks, it deals 4 damage to any target." (CR 608.2, 120.3).
fn deals_4_damage_any_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let source_0 = item.subject.object();
    let recipients_0 = t_0.clone();
    let Some(source) = source_0 else { return Ok(()) };
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (source, *t, 4u16))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        impending::REMOVE_A_TIME_COUNTER,
        triggered(
                when_asked(
                    EventMask(EventMask::ZONE_CHANGE.0 | EventMask::ATTACKS.0),
                    when_permanent_enters_attacks,
                ),
                deals_4_damage_any_target,
                "Whenever this permanent enters or attacks, it deals 4 damage to any target.",
            )
            .targets(&[targets::ANY]),
    ],
    statics: &[impending::NOT_A_CREATURE],
    enters: Some(impending::enters::<4>),
    ways: &[impending::way(mana("{2}{R}{R}"), "Impending 4—{2}{R}{R}")],
    ..oracle::OVERLORD_OF_THE_BOILERBILGES
};
