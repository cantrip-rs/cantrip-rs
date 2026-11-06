#[path = "../../cantrip-cards/src/cards/living_artifact.rs"]
mod living_artifact;

mod tests {
    use super::living_artifact::CARD;
    use cantrip_api::*;
    use cantrip_rules::{Game as RulesGame, Setup};
    use cantrip_state::{Frame, FrameKind, Span, INLINE_TARGETS};

    // Vorinclex's opponent counter-halving clause, isolated from its unrelated
    // creature abilities: replacements must see the full proposed quantity.
    fn opponent_puts(q: &dyn Query, a: &Asker, p: &Proposal) -> bool {
        matches!(p, Proposal::Counters(c) if q.state().opponents(a.controller).any(|opponent| opponent == c.putter))
    }
    fn half(_: &dyn Query, _: &Asker, p: &mut Proposal) {
        if let Proposal::Counters(c) = p { for n in &mut c.counts { *n /= 2; } }
    }
    static HALVER: CardDef = CardDef {
        name: "Opponent counter-halving fixture",
        types: TypeSet::of(&[CardType::Enchantment]),
        replacements: &[replacement(ReplacementKind::Counters, opponent_puts, half, "opponent puts half, rounded down")],
        ..CardDef::BLANK
    };

    // Resolve the real triggered ability with its bounded frozen damage payload.
    fn counters_after_damage(amount: u32, halved: bool) -> (i32, Vec<(i32, i32)>) {
        let cards = Box::leak(Box::new(CardTable::new([&CARD, &HALVER])));
        let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
        let you = PlayerId(0);
        let card = cards.id_of(&CARD);
        let aura = game.state.push_obj(Obj::new(card, you, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, aura);
        if halved {
            let halver = game.state.push_obj(Obj::new(cards.id_of(&HALVER), PlayerId(1), Zone::BATTLEFIELD));
            game.state.zone_push(Zone::BATTLEFIELD, halver);
        }
        let item = StackItem { event_amount: Some(amount),
            kind: StackKind::Triggered, object: aura, ability: AbilityRef { card, index: 0 }, controller: you,
            inline_targets: [Target::None; INLINE_TARGETS], target_count: 0, inline_spec_of: [0; INLINE_TARGETS],
            x: 0, kicked: 0, counted: 0, paid: 0, modes: 0, way: 0, choice: 0,
            subject: Target::Player(you), data: [0, 0], members: Span::default(), serial: 0,
            gift: None, shares: StackItem::NO_SHARES, spill: 0, exile_instead: ExileInstead::Never,
            flashback: false, attacking: false, since: 0, tapped_since: 0, copied: false, cast_known: u32::MAX,
            acquired: Acquired::OWN, instance: AbilityAcquisition::OWN, words: Words::NONE,
        };
        game.state.stack_push(item);
        let serial = game.state.stack.last().unwrap().serial;
        game.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
        game.run();
        let events = game.log.iter().filter_map(|record| match record.event {
            Event::CountersPut { object, kind: CounterKind::Vitality, amount, total, .. } if object == aura => Some((amount, total)),
            _ => None,
        }).collect();
        (game.state.obj(aura).counters(CounterKind::Vitality), events)
    }

    #[test]
    fn damage_counter_proposal_reaches_the_existing_physical_boundary_exactly() {
        assert_eq!(counters_after_damage(i32::MAX as u32, false), (i32::MAX, vec![(i32::MAX, i32::MAX)]));
    }

    #[test]
    fn counter_replacement_sees_the_full_damage_instead_of_a_prior_cap() {
        assert_eq!(counters_after_damage(32768, true), (16384, vec![(16384, 16384)]));
    }

    #[test]
    fn above_former_16_bit_counter_range_is_preserved() {
        assert_eq!(counters_after_damage(32768, false), (32768, vec![(32768, 32768)]));
    }
}
