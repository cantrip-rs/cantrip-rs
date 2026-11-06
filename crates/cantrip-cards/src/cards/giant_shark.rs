use cantrip_api::*;
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

/// The defending player, including a battle's protector, must control an Island (CR 508.5).
fn attack_rule(q: &dyn Query, asker: &Asker, attacker: ObjectId, defender: Target) -> bool {
    attacker != asker.source || defending_player_of(q.state(), defender)
        .is_none_or(|player| lands::controls_land_type(q, player, q.words(asker.source).subtype(Subtype::Island)))
}

/// "Whenever this creature blocks or becomes blocked by a creature that has been dealt damage this turn" (CR 603.2).
fn when_creature_blocks_becomes_blocked_by(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let id = *blocker;
    let other = *attacker;
    let subject = Target::Object(id);
    if asker.source == id
        && (q.chars(other).is(CardType::Creature)
            && ({
                let history_candidate = other;
                q
                    .history()
                    .this_turn()
                    .iter()
                    .filter_map(|r: &Record| -> Option<Fired> {
                        let event = &r.event;
                        if let Event::Damage { target, .. } = event {
                            let subject = *target;
                            if matches!(
                                * target, Target::Object(o) if history_candidate == o
                            ) {
                                return Fired::about(subject);
                            }
                        }
                        None
                    })
                    .count() as i64
            }) >= 1)
    {
        return Fired::about(subject);
    }
    None
}

/// "Whenever this creature blocks or becomes blocked by a creature that has been dealt damage this turn, this creature gets +2/+0 and gains trample until end of turn." (CR 608.2, 611.2a).
fn creature_gets_2_0_gains(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let kind_0_1_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Trample]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, 0),
                    Duration::EndOfTurn,
                );
                g.add_effect(item.object, object, kind_0_1_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

/// "Whenever this creature blocks or becomes blocked by a creature that has been dealt damage this turn" (CR 603.2).
fn when_creature_blocks_becomes_blocked_by_2(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Blocks { blocker, attacker, .. } = event else { return None };
    let id = *attacker;
    let other = *blocker;
    let subject = Target::Object(id);
    if asker.source == id
        && (q.chars(other).is(CardType::Creature)
            && ({
                let history_candidate = other;
                q
                    .history()
                    .this_turn()
                    .iter()
                    .filter_map(|r: &Record| -> Option<Fired> {
                        let event = &r.event;
                        if let Event::Damage { target, .. } = event {
                            let subject = *target;
                            if matches!(
                                * target, Target::Object(o) if history_candidate == o
                            ) {
                                return Fired::about(subject);
                            }
                        }
                        None
                    })
                    .count() as i64
            }) >= 1)
    {
        return Fired::about(subject);
    }
    None
}

/// "Whenever this creature blocks or becomes blocked by a creature that has been dealt damage this turn, this creature gets +2/+0 and gains trample until end of turn." (CR 608.2, 611.2a).
fn creature_gets_2_0_gains_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_1 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let kind_1_1_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Trample]));
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, 0),
                    Duration::EndOfTurn,
                );
                g.add_effect(item.object, object, kind_1_1_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

/// The controller's lack of Islands creates a state trigger (CR 603.8).
fn no_islands(q: &dyn Query, asker: &Asker) -> bool {
    !lands::controls_land_type(q, asker.controller, q.words(asker.source).subtype(Subtype::Island))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_becomes_blocked_by),
            creature_gets_2_0_gains,
            "Whenever this creature blocks or becomes blocked by a creature that has been dealt damage this turn, this creature gets +2/+0 and gains trample until end of turn.",
        ),
        triggered(
            when_asked(EventMask::BLOCKS, when_creature_blocks_becomes_blocked_by_2),
            creature_gets_2_0_gains_2,
            "Whenever this creature blocks or becomes blocked by a creature that has been dealt damage this turn, this creature gets +2/+0 and gains trample until end of turn.",
        ),
        state_triggered(
            no_islands,
            effects::sacrifice_self,
            "When you control no Islands, sacrifice this creature.",
        ),
    ],
    attack_rules: &[attack_rule],
    ..oracle::GIANT_SHARK
};
