use cantrip_api::*;
use cantrip_mechanics::targets::{CARD_IN_YOUR_GRAVEYARD, CREATURE_CARD_IN_YOUR_GRAVEYARD};
use cantrip_rules::{Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind, Span, INLINE_TARGETS};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static CREATURE: CardDef = CardDef {
    name: "Graveyard target creature", types: TypeSet::of(&[CardType::Creature]),
    power: Some(1), toughness: Some(1), ..CardDef::BLANK
};
static LAND: CardDef = CardDef {
    name: "Graveyard target land", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK
};

// Create and sacrifice a real token while resolving, before the next state-based actions.
fn probe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let token = g.create_tokens(g.card_id(&CREATURE), YOU, 1, false)?[0];
    g.end_sentence();
    let moved = g.sacrifice(YOU, &[token]);
    g.end_sentence();
    assert_eq!(moved.count(), 1);
    let in_graveyard = moved.objects()[0];
    assert!(g.obj(in_graveyard).token);
    assert_eq!(g.obj(in_graveyard).zone, Zone::graveyard(YOU));
    let asker = Asker::new(item.object, YOU);
    let any = CARD_IN_YOUR_GRAVEYARD.filter;
    let creature = CREATURE_CARD_IN_YOUR_GRAVEYARD.filter;
    assert!(!any(g, &asker, Target::Object(in_graveyard)), "a token is not a card");
    assert!(!creature(g, &asker, Target::Object(in_graveyard)), "a creature token is not a creature card");
    let own = g.state().zone(Zone::graveyard(YOU));
    let own_creature = own.iter().copied().find(|&id| !g.obj(id).token && g.obj(id).card == g.card_id(&CREATURE)).unwrap();
    let own_land = own.iter().copied().find(|&id| g.obj(id).card == g.card_id(&LAND)).unwrap();
    let opponent = g.state().zone(Zone::graveyard(THEM))[0];
    assert!(any(g, &asker, Target::Object(own_creature)));
    assert!(creature(g, &asker, Target::Object(own_creature)));
    assert!(any(g, &asker, Target::Object(own_land)));
    assert!(!creature(g, &asker, Target::Object(own_land)));
    g.return_to_hand(&[own_creature]);
    g.end_sentence();
    assert!(g.obj(own_creature).became.is_some());
    for id in [opponent, own_creature, token, item.object] {
        assert!(!any(g, &asker, Target::Object(id)));
        assert!(!creature(g, &asker, Target::Object(id)));
    }
    Ok(())
}
static SOURCE: CardDef = CardDef {
    name: "Graveyard target probe", types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[delayed_trigger(probe, "Probe graveyard targets")], ..CardDef::BLANK
};

#[test]
fn graveyard_card_targets_exclude_tokens_during_resolution() {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &CREATURE, &LAND])));
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    game.verify = true;
    let mut put = |def, owner, zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(def), owner, zone));
        game.state.zone_push(zone, id);
        id
    };
    let source = put(&SOURCE, YOU, Zone::BATTLEFIELD);
    put(&CREATURE, YOU, Zone::graveyard(YOU));
    put(&LAND, YOU, Zone::graveyard(YOU));
    put(&CREATURE, THEM, Zone::graveyard(THEM));
    game.state.stack_push(StackItem { target_choosers: Span::default(), instance: AbilityAcquisition::OWN, event_amount: None,
        kind: StackKind::Triggered, object: source, ability: AbilityRef::delayed(cards.id_of(&SOURCE), 0), controller: YOU,
        inline_targets: [Target::None; INLINE_TARGETS], target_count: 0, inline_spec_of: [0; INLINE_TARGETS],
        x: 0, kicked: 0, counted: 0, paid: 0, modes: 0, way: 0, choice: 0,
        subject: Target::None, data: [0, 0], members: Span::default(), serial: 0,
        gift: None, shares: StackItem::NO_SHARES, spill: 0, exile_instead: ExileInstead::Never,
        flashback: false, attacking: false, since: 0, tapped_since: 0, copied: false, cast_known: u32::MAX,
        acquired: Acquired::OWN, words: Words::NONE,
    });
    let serial = game.state.stack.last().unwrap().serial;
    game.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    game.run();
}
