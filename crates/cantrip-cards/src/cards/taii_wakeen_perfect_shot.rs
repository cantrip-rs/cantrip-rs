use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever a source you control deals noncombat damage to a creature equal to that creature's toughness" (CR 120.3, 603.2,
/// 603.2c): damage a source its controller controlled as it dealt it (a spell, an ability's source or a permanent, even one
/// that has since left; `Event::Damage::controller`), not combat damage, dealt to a creature, of an amount - once replaced and
/// prevented - that is the creature's toughness as the damage is dealt. Damage already marked is not counted (the 2024-04-12
/// ruling), and a creature of no toughness takes no damage equal to it. Each source's damage is its own trigger (CR 603.2c).
fn source_you_control_deals_noncombat_damage_equal_to_a_creatures_toughness(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Object(creature), amount, kind, controller, .. }
            if !kind.is_combat()
                && *controller == me.controller
                && q.chars(*creature).is(CardType::Creature)
                && q.chars(*creature).toughness.is_some_and(|toughness| i64::from(*amount) == i64::from(toughness)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "If a source you control would deal noncombat damage to a permanent or player this turn" (CR 614.1, 120.3): damage of any
/// source this effect's player controls, not combat damage, to a permanent or a player. Damage already prevented to nothing is no
/// longer proposed, so there is nothing to add to (CR 615.6; the 2024-04-12 ruling).
fn noncombat_damage_you_deal(q: &dyn Query, asker: &Asker, _: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if !d.combat && q.controller(d.source) == asker.controller && matches!(d.target, Target::Object(_) | Target::Player(_)))
}

/// "... it deals that much damage plus X instead" (CR 614.1): the X this activation fixed as it resolved (CR 107.3a,
/// `Query::replacement_value`), added to the damage of the same source to the same recipient (the 2024-04-12 ruling), as the
/// effects applied before it have left it (CR 616.1f).
fn plus_x(q: &dyn Query, _: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount += q.replacement_value();
    }
}

/// "{X}, {T}: If a source you control would deal noncombat damage ... this turn" is a lasting replacement effect (CR 611.2a)
/// that keeps this activation's X (CR 107.3a, 608.2h): two activations each add their own.
fn activate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(
        item.controller,
        item.object,
        EffectKind::ReplacementRule { card: item.ability.card, index: 0, value: item.x, player: None },
        Duration::EndOfTurn,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DAMAGE, source_you_control_deals_noncombat_damage_equal_to_a_creatures_toughness),
            |g, item| effects::draw(g, item, 1),
            "Whenever a source you control deals noncombat damage to a creature equal to that creature's toughness, draw a card.",
        ),
        ability(
            cost("{X}").and(TAP),
            activate,
            "{X}, {T}: If a source you control would deal noncombat damage to a permanent or player this turn, it deals that much damage plus X instead.",
        ),
    ],
    granted_replacements: &[granted_replacement(
        ReplacementKind::Damage,
        noncombat_damage_you_deal,
        plus_x,
        "deals that much damage plus X instead",
    )
    .show_captured_value()
    .text("If a source you control would deal noncombat damage to a permanent or player this turn, it deals that much damage plus X instead.")],
    ..oracle::TAII_WAKEEN_PERFECT_SHOT
};
