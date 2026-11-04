use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "This enchantment can't be the target of white spells or abilities from white sources. Sacrifice an Orc: Each player may tap any number of untapped white creatures they control. For each creature tapped this way, that player chooses up to two Plains. Then destroy all Plains that weren't chosen this way by any player." (CR 604.1).
fn player_tap_any_number_untapped_rule_on(
    q: &dyn LayerQuery,
    source: ObjectId,
    subject: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    subject == asker.source
}

/// "This enchantment can't be the target of white spells or abilities from white sources. Sacrifice an Orc: Each player may tap any number of untapped white creatures they control. For each creature tapped this way, that player chooses up to two Plains. Then destroy all Plains that weren't chosen this way by any player." (CR 604.1).
fn player_tap_any_number_untapped_rule_by(
    q: &dyn LayerQuery,
    source: ObjectId,
    by: &Asker,
    by_source: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    by.spell && (*by_source).colors.has(q.words(asker.source).color(Color::White))
}

/// "This enchantment can't be the target of white spells or abilities from white sources. Sacrifice an Orc: Each player may tap any number of untapped white creatures they control. For each creature tapped this way, that player chooses up to two Plains. Then destroy all Plains that weren't chosen this way by any player." (CR 604.1).
fn player_tap_any_number_untapped_rule_on_2(
    q: &dyn LayerQuery,
    source: ObjectId,
    subject: ObjectId,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    subject == asker.source
}

/// "This enchantment can't be the target of white spells or abilities from white sources. Sacrifice an Orc: Each player may tap any number of untapped white creatures they control. For each creature tapped this way, that player chooses up to two Plains. Then destroy all Plains that weren't chosen this way by any player." (CR 604.1).
fn player_tap_any_number_untapped_rule_by_2(
    q: &dyn LayerQuery,
    source: ObjectId,
    by: &Asker,
    by_source: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    !(by.spell)
        && (!by.spell
            && (*by_source).colors.has(q.words(asker.source).color(Color::White)))
}

/// The color is captured from the resolving ability's text, including text changes before activation (CR 113.7a).
fn untapped_creature_of_color(q: &dyn Query, choice: &cantrip_api::select::Choice, color: u32) -> cantrip_api::select::Verdict {
    cantrip_api::select::Verdict::each(choice, |id| {
        q.controller(id) == choice.chooser && !q.obj(id).tapped && q.chars(id).is(CardType::Creature)
            && q.chars(id).colors.has(Color::ALL[color as usize])
    })
}

fn has_land_subtype(q: &dyn Query, choice: &cantrip_api::select::Choice, subtype: u32) -> cantrip_api::select::Verdict {
    cantrip_api::select::Verdict::each(choice, |id| q.chars(id).has_subtype(Subtype::ALL[subtype as usize]))
}

/// The players choose in turn order, tap simultaneously, then choose Plains before the rest are destroyed (CR 101.4, 701.8a, 701.26a).
fn player_tap_any_number_untapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players: Vec<_> = g.state().apnap().collect();
    let color = g.words(item.object).color(Color::White) as u32;
    let plains = g.words(item.object).subtype(Subtype::Plains) as u32;
    let creature_kind = ["untapped white creature", "untapped blue creature", "untapped black creature", "untapped red creature", "untapped green creature"][color as usize];
    let land_names = [(Subtype::Plains, "Plains land"), (Subtype::Island, "Island land"), (Subtype::Swamp, "Swamp land"), (Subtype::Mountain, "Mountain land"), (Subtype::Forest, "Forest land")];
    let land_type = Subtype::ALL[plains as usize];
    let land_kind = land_names.iter().find(|(kind, _)| *kind == land_type).map_or(land_type.name(), |(_, name)| *name);
    let mut chosen_by_player = Vec::new();
    let mut all_tapped = Vec::new();
    for &player in &players {
        let candidates = g.state().battlefield().to_vec();
        let rule = select::Selection::new(select::ANY_NUMBER)
            .with(select::part(&(untapped_creature_of_color as cantrip_api::select::SelectFn), color, "you control"))
            .called(creature_kind);
        let chosen = choose::select(g, player, item.object, Purpose::Tap, candidates, rule)?;
        all_tapped.extend(chosen.iter().copied());
        chosen_by_player.push(chosen.len());
    }
    g.end_sentence();
    g.tap_together(&all_tapped);
    g.end_sentence();
    let mut saved = Vec::new();
    for (&player, &count) in players.iter().zip(&chosen_by_player) {
        let candidates = g.state().battlefield().to_vec();
        let limit = u32::try_from(count.saturating_mul(2)).unwrap_or(u32::MAX);
        let rule = select::Selection::new(select::up_to(limit))
            .with(select::part(&(has_land_subtype as cantrip_api::select::SelectFn), plains, ""))
            .called(land_kind);
        saved.extend(choose::select(g, player, item.object, Purpose::Choose, candidates, rule)?);
    }
    g.end_sentence();
    let doomed: Vec<_> = g.state().battlefield().iter().copied()
        .filter(|&id| g.chars(id).has_subtype(Subtype::ALL[plains as usize]) && !saved.contains(&id))
        .collect();
    g.destroy_all(&doomed);
    Ok(())
}

/// "Sacrifice an Orc: Each player may tap any number of untapped white creatures they control. For each creature tapped this way, that player chooses up to two Plains. Then destroy all Plains that weren't chosen this way by any player." (CR 118.1).
fn player_tap_any_number_untapped_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Orc))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            sacrifice(1, player_tap_any_number_untapped_2),
            player_tap_any_number_untapped,
            "Sacrifice an Orc: Each player may tap any number of untapped white creatures they control. For each creature tapped this way, that player chooses up to two Plains. Then destroy all Plains that weren't chosen this way by any player.",
        ),
    ],
    targeting_rules: &[
        TargetingRule {
            kind: TargetingRuleKind::Forbid(player_tap_any_number_untapped_rule_by),
            applies: player_tap_any_number_untapped_rule_on,
        },
        TargetingRule {
            kind: TargetingRuleKind::Forbid(player_tap_any_number_untapped_rule_by_2),
            applies: player_tap_any_number_untapped_rule_on_2,
        },
    ],
    ..oracle::RAIDING_PARTY
};
