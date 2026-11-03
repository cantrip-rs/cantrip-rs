use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "+1: Until your next turn, prevent all damage target permanent would deal. 0: Until end of turn, Gideon becomes a 4/4 Human Soldier creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn." (CR 615.1).
fn made_1_applies(
    q: &dyn Query,
    _: &Asker,
    affected: ObjectId,
    e: &DamageEvent,
) -> bool {
    {
        let source = e.source;
        if !q.state().obj(source).zone.is(ZoneKind::Stack) && affected == source {
            return true;
        }
    }
    false
}

/// "+1: Until your next turn, prevent all damage target permanent would deal." (CR 608.2, 611.2a).
fn next_prevent_all_damage_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    {
        let to = Some(Target::None);
        let about = t_0.get(0).copied().and_then(|t: Target| t.object());
        if let (Some(to), Some(about)) = (to, about) {
            g.add_effect(
                item.object,
                about,
                EffectKind::Prevention {
                    card: item.ability.card,
                    index: 0,
                    to,
                    shield: Shield::All,
                },
                Duration::UntilNextTurnOf(item.controller),
            );
        }
    }
    Ok(())
}

/// "+1: Until your next turn, prevent all damage target permanent would deal. 0: Until end of turn, Gideon becomes a 4/4 Human Soldier creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn." (CR 615.1).
fn made_4_applies(_: &dyn Query, _: &Asker, _: ObjectId, _: &DamageEvent) -> bool {
    true
}

/// "0: Until end of turn, Gideon becomes a 4/4 Human Soldier creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn." (CR 608.2, 611.2a).
fn gideon_becomes_4_4_human(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_2 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let kind_2_1_0 = {
        let q: &dyn Query = g;
        EffectKind::AddSubtype(q.words(item.object).subtype(Subtype::Human))
    };
    let kind_2_2_0 = {
        let q: &dyn Query = g;
        EffectKind::AddSubtype(q.words(item.object).subtype(Subtype::Soldier))
    };
    let kind_2_3_0 = EffectKind::AddTypes(TypeSet::of(&[CardType::Creature]));
    let kind_2_4_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Indestructible]));
    instruction(
        g,
        |g| {
            for &object in &objects_2 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::SetPt(4, 4),
                    Duration::EndOfTurn,
                );
                g.add_effect(item.object, object, kind_2_1_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_2_2_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_2_3_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_2_4_0, Duration::EndOfTurn);
            }
        },
    );
    g.end_sentence();
    {
        let to = Some(item.object)
            .map(Target::Object)
            .into_iter()
            .collect::<Vec<Target>>()
            .first()
            .copied();
        let about = to.and_then(|to: Target| to.object()).or(Some(item.object));
        if let (Some(to), Some(about)) = (to, about) {
            g.add_effect(
                item.object,
                about,
                EffectKind::Prevention {
                    card: item.ability.card,
                    index: 1,
                    to,
                    shield: Shield::All,
                },
                Duration::EndOfTurn,
            );
        }
    }
    Ok(())
}

/// "0: You get an emblem with "As long as you control a Gideon planeswalker, you can't lose the game and your opponents can't win the game."" (CR 604.1).
fn get_emblem_as_long_as_rule(
    q: &dyn Query,
    asker: &Asker,
    player: PlayerId,
    _: LossReason,
) -> bool {
    (q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            q.controller(id) == Some(asker.controller).unwrap_or(PlayerId(u8::MAX))
                && (q
                    .chars(id)
                    .has_subtype(q.words(asker.source).subtype(Subtype::Gideon))
                    && q.chars(id).is(CardType::Planeswalker))
        })
        .count() >= 1) && player == asker.controller
}

/// "0: You get an emblem with "As long as you control a Gideon planeswalker, you can't lose the game and your opponents can't win the game."" (CR 604.1).
fn get_emblem_as_long_as_rule_2(q: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    (q
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            q.controller(id) == Some(asker.controller).unwrap_or(PlayerId(u8::MAX))
                && (q
                    .chars(id)
                    .has_subtype(q.words(asker.source).subtype(Subtype::Gideon))
                    && q.chars(id).is(CardType::Planeswalker))
        })
        .count() >= 1) && player != asker.controller
}

pub static EMBLEM: CardDef = CardDef {
    name: "Gideon of the Trials emblem",
    has_cost: false,
    loss_rules: &[get_emblem_as_long_as_rule],
    win_rules: &[get_emblem_as_long_as_rule_2],
    ..CardDef::BLANK
};

/// "0: You get an emblem with "As long as you control a Gideon planeswalker, you can't lose the game and your opponents can't win the game."" (CR 608.2).
fn get_emblem_as_long_as(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                loyalty(1),
                next_prevent_all_damage_target,
                "+1: Until your next turn, prevent all damage target permanent would deal.",
            )
            .targets(&[targets::PERMANENT])
            .sorcery_speed(),
        ability(
                loyalty(0),
                gideon_becomes_4_4_human,
                "0: Until end of turn, Gideon becomes a 4/4 Human Soldier creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn.",
            )
            .sorcery_speed(),
        ability(
                loyalty(0),
                get_emblem_as_long_as,
                "0: You get an emblem with \"As long as you control a Gideon planeswalker, you can't lose the game and your opponents can't win the game.\"",
            )
            .sorcery_speed(),
    ],
    granted_preventions: &[
        GrantedPreventionDef {
            hint: "+1: Until your next turn, prevent all damage target permanent would deal.",
            damage: "damage",
            of_it: None,
            applies: made_1_applies,
            then: None,
            text: "Until your next turn, prevent all damage target permanent would deal.",
        },
        GrantedPreventionDef {
            hint: "0: Until end of turn, Gideon becomes a 4/4 Human Soldier creature with indestructible that's still a planeswalker. Prevent all damage that would be dealt to him this turn.",
            damage: "damage",
            of_it: None,
            applies: made_4_applies,
            then: None,
            text: "Prevent all damage that would be dealt to him this turn.",
        },
    ],
    ..oracle::GIDEON_OF_THE_TRIALS
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];
