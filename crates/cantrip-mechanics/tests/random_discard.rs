use cantrip_api::*;
use cantrip_rules::{Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind, Span, INLINE_TARGETS};
use std::collections::BTreeMap;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::discard_at_random(g, YOU, item.data[0] as usize);
    Ok(())
}

static SOURCE: CardDef = CardDef {
    name: "Random discard test source", types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[delayed_trigger(discard, "Discard cards at random.")], ..CardDef::BLANK
};
static CARD: CardDef = CardDef {
    name: "Random discard test card", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK
};

fn resolve(size: usize, count: usize, faces: &[u8]) -> (RulesGame, Vec<ObjectId>) {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &CARD])));
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    game.verify = true;
    let source = game.state.push_obj(Obj::new(cards.id_of(&SOURCE), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    let mut original = Vec::new();
    for _ in 0..size {
        let id = game.state.push_obj(Obj::new(cards.id_of(&CARD), YOU, Zone::hand(YOU)));
        game.state.zone_push(Zone::hand(YOU), id);
        original.push(id);
    }
    let other = game.state.push_obj(Obj::new(cards.id_of(&CARD), THEM, Zone::hand(THEM)));
    game.state.zone_push(Zone::hand(THEM), other);
    let item = StackItem { instance: AbilityAcquisition::OWN, event_amount: None,
        kind: StackKind::Triggered, object: source, ability: AbilityRef::delayed(cards.id_of(&SOURCE), 0), controller: YOU,
        inline_targets: [Target::None; INLINE_TARGETS], target_count: 0, inline_spec_of: [0; INLINE_TARGETS],
        x: 0, kicked: 0, counted: 0, paid: 0, modes: 0, way: 0, choice: 0,
        subject: Target::None, data: [count as u32, 0], members: Span::default(), serial: 0,
        gift: None, shares: StackItem::NO_SHARES, spill: 0, exile_instead: ExileInstead::Never,
        flashback: false, attacking: false, since: 0, tapped_since: 0, copied: false, cast_known: u32::MAX,
        acquired: Acquired::OWN, words: Words::NONE,
    };
    game.state.force_rolls(faces);
    game.state.stack_push(item);
    let serial = game.state.stack.last().unwrap().serial;
    game.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    game.run();
    assert_eq!(game.state.zone(Zone::hand(THEM)), &[other]);
    assert_eq!(game.state.zone(Zone::hand(YOU)).len(), size.saturating_sub(count));
    assert_eq!(game.log.iter().filter(|r| matches!(r.event, Event::Discarded { player: YOU, .. })).count(), size.min(count));
    (game, original)
}

fn choices(game: &RulesGame) -> Vec<(u32, u32)> {
    game.log.iter().filter_map(|r| match r.event {
        Event::ChosenAtRandom { count, index } => Some((count, index)), _ => None,
    }).collect()
}

#[test]
fn fixed_choices_can_select_each_card_and_are_recorded() {
    for face in 1..=3 {
        let (game, hand) = resolve(3, 1, &[face]);
        assert!(!game.state.zone(Zone::hand(YOU)).contains(&hand[usize::from(face - 1)]));
        assert_eq!(choices(&game), vec![(3, u32::from(face - 1))]);
        assert!(!game.log.iter().any(|r| matches!(r.event, Event::DieRolled { .. })));
    }
}

#[test]
fn every_two_card_subset_has_the_same_number_of_forced_choice_paths() {
    // Enumerate the finite choices, not RNG seeds: 4*3 ordered paths, six subsets.
    let mut subsets = BTreeMap::new();
    for first in 1..=4 {
        for second in 1..=3 {
            let (game, hand) = resolve(4, 2, &[first, second]);
            let removed: Vec<usize> = hand.iter().enumerate().filter_map(|(i, id)|
                (!game.state.zone(Zone::hand(YOU)).contains(id)).then_some(i)).collect();
            assert_eq!(removed.len(), 2);
            *subsets.entry(removed).or_insert(0) += 1;
            assert_eq!(choices(&game), vec![(4, u32::from(first - 1)), (3, u32::from(second - 1))]);
        }
    }
    assert_eq!(subsets.len(), 6);
    assert!(subsets.values().all(|count| *count == 2));
}

#[test]
fn discarding_the_whole_hand_or_nothing_makes_no_random_choice() {
    for (size, count) in [(0, 0), (0, 4), (1, 1), (1, 9), (3, 0), (3, 3), (3, 10)] {
        let (mut game, _) = resolve(size, count, &[2]);
        assert!(choices(&game).is_empty());
        assert_eq!(game.state.roll_die(6), 2, "no random choice should consume a forced face");
    }
}

#[test]
fn larger_hands_still_choose_distinct_cards_and_record_each_choice() {
    let (game, _) = resolve(23, 3, &[]);
    let choices = choices(&game);
    assert_eq!(choices.iter().map(|c| c.0).collect::<Vec<_>>(), vec![23, 22, 21]);
    assert!(choices.iter().all(|&(count, index)| index < count));
}

#[test]
fn all_choices_precede_the_single_discard_instruction() {
    let (game, _) = resolve(4, 2, &[1, 1]);
    let last_choice = game.log.iter().rposition(|r| matches!(r.event, Event::ChosenAtRandom { .. })).unwrap();
    let discarded: Vec<_> = game.log.iter().enumerate().filter(|(_, r)| matches!(r.event, Event::Discarded { .. })).collect();
    assert!(discarded.iter().all(|(index, _)| *index > last_choice));
    assert_eq!(discarded[0].1.cause, discarded[1].1.cause);
}
