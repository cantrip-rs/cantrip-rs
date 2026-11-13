//! Shared selectors preserve the printed type restrictions, controller independence and text changes.
use cantrip_api::*;
use cantrip_mechanics::{lands, targets};
use cantrip_rules::{Game as RulesGame, Setup};
use cantrip_state::{Frame, FrameKind, Span, INLINE_TARGETS};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static WALL: CardDef = CardDef {
    name: "Selector Wall", types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Wall]), power: Some(1), toughness: Some(3), ..CardDef::BLANK
};
static ELF: CardDef = CardDef {
    name: "Selector Elf", subtypes: SubtypeSet::of(&[Subtype::Elf]), ..WALL
};
static NONCREATURE_WALL: CardDef = CardDef {
    name: "Noncreature Wall", types: TypeSet::of(&[CardType::Kindred, CardType::Enchantment]),
    power: None, toughness: None, ..WALL
};
static WALKER: CardDef = CardDef {
    name: "Selector walker", keywords: Keywords::of(&[Keyword::SnowForestwalk, Keyword::Mountainwalk]), ..ELF
};
static NONCREATURE_WALKER: CardDef = CardDef {
    name: "Noncreature walker", keywords: Keywords::of(&[Keyword::Forestwalk]), ..NONCREATURE_WALL
};

fn find(g: &dyn Game, def: &'static CardDef, zone: Zone, owner: PlayerId) -> ObjectId {
    g.state().zone(zone).iter().copied().find(|&id| g.obj(id).card == g.card_id(def) && g.obj(id).owner == owner).unwrap()
}

fn probe(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // Query another permanent: a resolving ability retains its own captured words (CR 612.2).
    let rule_source = find(g, &RULE_SOURCE, Zone::BATTLEFIELD, YOU);
    let asker = Asker::new(rule_source, YOU);
    let own_wall = find(g, &WALL, Zone::BATTLEFIELD, YOU);
    let opposing_wall = find(g, &WALL, Zone::BATTLEFIELD, THEM);
    let graveyard_wall = find(g, &WALL, Zone::graveyard(YOU), YOU);
    let elf = find(g, &ELF, Zone::BATTLEFIELD, YOU);
    let noncreature = find(g, &NONCREATURE_WALL, Zone::BATTLEFIELD, YOU);
    let wall = targets::WALL_CREATURE.filter;
    assert!(wall(g, &asker, Target::Object(own_wall)));
    assert!(wall(g, &asker, Target::Object(opposing_wall)));
    for target in [Target::Object(graveyard_wall), Target::Object(elf), Target::Object(noncreature), Target::Player(THEM), Target::None] {
        assert!(!wall(g, &asker, target), "only Wall creatures on the battlefield qualify");
    }

    let walker = find(g, &WALKER, Zone::BATTLEFIELD, YOU);
    let noncreature_walker = find(g, &NONCREATURE_WALKER, Zone::BATTLEFIELD, YOU);
    let forest = lands::named_landwalk_exception::<{ lands::sub(Subtype::Forest) }>;
    let mountain = lands::named_landwalk_exception::<{ lands::sub(Subtype::Mountain) }>;
    let island = lands::named_landwalk_exception::<{ lands::sub(Subtype::Island) }>;
    assert_eq!(forest(g, &asker, own_wall, walker), Keywords::of(&[Keyword::Forestwalk]), "the exception includes snow forestwalk");
    assert_eq!(mountain(g, &asker, own_wall, walker), Keywords::of(&[Keyword::Mountainwalk]));
    assert_eq!(island(g, &asker, own_wall, walker), Keywords::NONE);
    assert_eq!(forest(g, &asker, own_wall, noncreature_walker), Keywords::NONE);
    assert_eq!(forest(g, &asker, own_wall, elf), Keywords::NONE);

    g.add_effect(item.object, rule_source, EffectKind::ChangeText { from: Word::Subtype(Subtype::Wall), to: Word::Subtype(Subtype::Elf) }, Duration::EndOfTurn);
    g.end_sentence();
    assert!(wall(g, &asker, Target::Object(elf)), "the source now says Elf creature");
    assert!(!wall(g, &asker, Target::Object(own_wall)));
    g.add_effect(item.object, rule_source, EffectKind::ChangeText { from: Word::Subtype(Subtype::Forest), to: Word::Subtype(Subtype::Mountain) }, Duration::EndOfTurn);
    g.end_sentence();
    assert_eq!(forest(g, &asker, own_wall, walker), Keywords::of(&[Keyword::Mountainwalk]), "the exception reads its source's changed land word");
    Ok(())
}

static SOURCE: CardDef = CardDef {
    name: "Shared selector probe", types: TypeSet::of(&[CardType::Artifact]),
    delayed: &[delayed_trigger(probe, "Probe shared selectors")], ..CardDef::BLANK
};
static RULE_SOURCE: CardDef = CardDef { name: "Shared selector rule source", ..SOURCE };

#[test]
fn wall_targets_and_named_landwalk_exceptions_preserve_their_rules() {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &RULE_SOURCE, &WALL, &ELF, &NONCREATURE_WALL, &WALKER, &NONCREATURE_WALKER])));
    let mut game = RulesGame::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    game.verify = true;
    let mut put = |def, owner, zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(def), owner, zone));
        game.state.zone_push(zone, id);
        id
    };
    let source = put(&SOURCE, YOU, Zone::BATTLEFIELD);
    put(&RULE_SOURCE, YOU, Zone::BATTLEFIELD);
    for def in [&WALL, &ELF, &NONCREATURE_WALL, &WALKER, &NONCREATURE_WALKER] { put(def, YOU, Zone::BATTLEFIELD); }
    put(&WALL, THEM, Zone::BATTLEFIELD);
    put(&WALL, YOU, Zone::graveyard(YOU));
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
