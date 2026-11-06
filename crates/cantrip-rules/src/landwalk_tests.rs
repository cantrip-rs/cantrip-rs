//! Forced landwalk positions: CR 702.14, 612.2, 613.1d/f. No random game play.
use super::*;
use crate::Setup;
use cantrip_api::Game as Rules;
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
// Expected rules data, independent of the core's mapping table.
const SNOW_CASES: [(Keyword, Subtype); 5] = [
    (Keyword::SnowPlainswalk, Subtype::Plains), (Keyword::SnowIslandwalk, Subtype::Island),
    (Keyword::SnowSwampwalk, Subtype::Swamp), (Keyword::SnowMountainwalk, Subtype::Mountain),
    (Keyword::SnowForestwalk, Subtype::Forest),
];
static CREATURE: CardDef = CardDef {
    name: "Walker", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK
};
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}
fn position(keyword: Keyword, land_type: Option<Subtype>, supertype: Supertypes, land_owner: PlayerId) -> (Game, ObjectId, ObjectId, ObjectId) {
    let land = Box::leak(Box::new(CardDef { name: "Walked land", types: TypeSet::of(&[CardType::Land]),
        subtypes: land_type.map_or(SubtypeSet::NONE, |t| SubtypeSet::NONE.with(t)), supertypes: supertype, ..CardDef::BLANK }));
    let walker = Box::leak(Box::new(CardDef { name: "Attacking walker", keywords: Keywords::of(&[keyword]), ..CREATURE }));
    let cards = Box::leak(Box::new(CardTable::new([walker, &CREATURE, land, &SNOW, &BASIC, &LEGENDARY])));
    let deck = vec![cards.id_of(&CREATURE); 20];
    let mut g = Game::new(cards, &Setup { decks: vec![deck; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Settle));
    g.state.update_turn(|t| { t.active = YOU; t.number = 1; t.set_step(Step::DeclareBlockers); });
    let mut put = |card, owner| {
        let id = g.state.push_obj(Obj::new(card, owner, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id); id
    };
    let attacker = put(cards.id_of(walker), YOU);
    let blocker = put(cards.id_of(&CREATURE), THEM);
    let land = put(cards.id_of(land), land_owner);
    (g, attacker, blocker, land)
}
fn supertype(basic: bool, legendary: bool, snow: bool) -> Supertypes { Supertypes { basic, legendary, snow } }
fn blockable(g: &mut Game, attacker: ObjectId, blocker: ObjectId) -> bool { can_block(&cx(g), blocker, attacker, &[]) }

#[test]
fn snow_basic_landwalk_requires_both_current_snow_and_the_correct_land_type() {
    for (walk, land_type) in SNOW_CASES {
        for (actual, snow, blocks) in [(land_type, true, false), (land_type, false, true), (Subtype::Desert, true, true)] {
            let (mut g, a, b, _) = position(walk, Some(actual), supertype(true, false, snow), THEM);
            assert_eq!(blockable(&mut g, a, b), blocks, "{walk:?}, {actual:?}, snow {snow}");
            assert!(!cx(&mut g).chars(a).has(Keyword::landwalk(land_type).unwrap()), "raw keyword membership stays exact");
        }
        let (mut g, a, b, _) = position(walk, Some(land_type), supertype(true, false, true), YOU);
        assert!(blockable(&mut g, a, b), "only the defender's lands count");
    }
}

#[test]
fn all_landwalk_qualifiers_require_a_land_and_the_current_qualifier() {
    for (walk, subtype, yes, no) in [
        (Keyword::Desertwalk, Some(Subtype::Desert), supertype(false, false, false), None),
        (Keyword::SnowLandwalk, None, supertype(false, false, true), Some(supertype(false, false, false))),
        (Keyword::NonbasicLandwalk, Some(Subtype::Forest), supertype(false, false, false), Some(supertype(true, false, false))),
        (Keyword::LegendaryLandwalk, None, supertype(false, true, false), Some(supertype(false, false, false))),
    ] {
        let (mut g, a, b, land) = position(walk, subtype, yes, THEM);
        assert!(!blockable(&mut g, a, b), "{walk:?} qualifies");
        cx(&mut g).add_effect(land, land, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])), Duration::EndOfTurn);
        assert!(blockable(&mut g, a, b), "{walk:?} requires the land card type");
        let (mut g, a, b, _) = position(walk, if no.is_none() { Some(Subtype::Island) } else { subtype }, no.unwrap_or(yes), THEM);
        assert!(blockable(&mut g, a, b), "{walk:?} wrong qualifier");
    }
}

#[test]
fn ordinary_landwalk_works_through_snow_and_non_snow_lands() {
    for (walk, land_type) in Keyword::LANDWALKS {
        for snow in [false, true] {
            let (mut g, a, b, _) = position(walk, Some(land_type), supertype(true, false, snow), THEM);
            assert!(!blockable(&mut g, a, b));
        }
    }
}

#[test]
fn text_changes_preserve_snow_and_change_blocking_eligibility() {
    let (mut g, a, b, _) = position(Keyword::SnowForestwalk, Some(Subtype::Island), supertype(true, false, false), THEM);
    cx(&mut g).add_effect(a, a, EffectKind::ChangeText { from: Word::Subtype(Subtype::Forest), to: Word::Subtype(Subtype::Island) }, Duration::EndOfTurn);
    assert_eq!(cx(&mut g).chars(a).keywords, Keywords::of(&[Keyword::SnowIslandwalk]));
    assert!(blockable(&mut g, a, b), "an ordinary Island still does not qualify");
    let (mut g, a, b, _) = position(Keyword::SnowForestwalk, Some(Subtype::Island), supertype(true, false, true), THEM);
    assert!(blockable(&mut g, a, b));
    cx(&mut g).add_effect(a, a, EffectKind::ChangeText { from: Word::Subtype(Subtype::Forest), to: Word::Subtype(Subtype::Island) }, Duration::EndOfTurn);
    assert!(!blockable(&mut g, a, b));
}

#[test]
fn named_removal_includes_snow_but_granting_is_exact_and_later_grants_survive() {
    let (mut g, a, b, _) = position(Keyword::SnowForestwalk, Some(Subtype::Forest), supertype(true, false, true), THEM);
    cx(&mut g).add_effect(a, a, EffectKind::LoseKeywords(Keywords::of(&[Keyword::Forestwalk])), Duration::EndOfTurn);
    assert!(!cx(&mut g).chars(a).keywords.has_landwalk());
    assert!(blockable(&mut g, a, b));
    cx(&mut g).add_effect(a, a, EffectKind::AddKeywords(Keywords::of(&[Keyword::Forestwalk])), Duration::EndOfTurn);
    assert_eq!(cx(&mut g).chars(a).keywords, Keywords::of(&[Keyword::Forestwalk]));
    assert!(!blockable(&mut g, a, b));
    cx(&mut g).add_effect(a, a, EffectKind::LoseKeywords(Keywords::LANDWALKS), Duration::EndOfTurn);
    assert!(!cx(&mut g).chars(a).keywords.has_landwalk());
}

#[test]
fn a_lands_type_and_controller_changes_update_snow_walk_eligibility() {
    let (mut g, a, b, land) = position(Keyword::SnowForestwalk, Some(Subtype::Island), supertype(true, false, true), THEM);
    assert!(blockable(&mut g, a, b));
    cx(&mut g).add_effect(land, land, EffectKind::AddSubtype(Subtype::Forest), Duration::EndOfTurn);
    assert!(!blockable(&mut g, a, b));
    cx(&mut g).add_effect(land, land, EffectKind::Control(YOU), Duration::EndOfTurn);
    assert!(blockable(&mut g, a, b));
}

fn lands(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool { c.is(CardType::Land) }
fn snow(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) { c.supertypes.snow = true; }
fn basic(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) { c.supertypes.basic = true; }
fn legendary(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) { c.supertypes.legendary = true; }
static SNOW: CardDef = CardDef { name: "Snow setter", types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, lands, snow, "Lands are snow.")], ..CardDef::BLANK };
static BASIC: CardDef = CardDef { name: "Basic setter", types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, lands, basic, "Lands are basic.")], ..CardDef::BLANK };
static LEGENDARY: CardDef = CardDef { name: "Legendary setter", types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Type, lands, legendary, "Lands are legendary.")], ..CardDef::BLANK };

#[test]
fn supertype_changes_are_read_from_the_current_layers_in_both_directions() {
    for (walk, setter, before) in [(Keyword::SnowForestwalk, &SNOW, true), (Keyword::SnowLandwalk, &SNOW, true),
        (Keyword::LegendaryLandwalk, &LEGENDARY, true), (Keyword::NonbasicLandwalk, &BASIC, false)] {
        let (mut g, a, b, _) = position(walk, Some(Subtype::Forest), supertype(false, false, false), THEM);
        assert_eq!(blockable(&mut g, a, b), before);
        let id = g.state.push_obj(Obj::new(g.cards.id_of(setter), YOU, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, id);
        g.state.see_statics();
        assert_eq!(blockable(&mut g, a, b), !before, "{walk:?} gains its new qualifier");
        cx(&mut g).return_to_hand(&[id]);
        assert_eq!(blockable(&mut g, a, b), before, "{walk:?} loses the qualifier again");
    }
}

#[test]
fn each_snow_land_type_changes_without_losing_the_snow_restriction() {
    for (walk, from) in SNOW_CASES {
        for (new, to) in SNOW_CASES {
            let (mut g, a, _, _) = position(walk, None, supertype(false, false, false), THEM);
            cx(&mut g).add_effect(a, a, EffectKind::ChangeText { from: Word::Subtype(from), to: Word::Subtype(to) }, Duration::EndOfTurn);
            assert_eq!(cx(&mut g).chars(a).keywords, Keywords::of(&[new]));
        }
    }
}

#[test]
fn high_keyword_bits_survive_rollback_and_change_loop_identity() {
    let (mut g, a, _, _) = position(Keyword::SnowForestwalk, None, supertype(false, false, false), THEM);
    let before = crate::loops::position(&g.state);
    let mark = g.state.mark();
    cx(&mut g).add_effect(a, a, EffectKind::AddKeywords(Keywords::of(&[Keyword::Desertwalk])), Duration::EndOfTurn);
    let desert = crate::loops::position(&g.state);
    assert_ne!(desert, before);
    assert!(cx(&mut g).chars(a).has(Keyword::Desertwalk));
    g.state.rollback(mark);
    // Same effect structure and timestamp, differing only in bits above 63: both high words must be hashed.
    cx(&mut g).add_effect(a, a, EffectKind::AddKeywords(Keywords::of(&[Keyword::SnowLandwalk])), Duration::EndOfTurn);
    assert_ne!(crate::loops::position(&g.state), desert);
    g.state.rollback(mark);
    assert_eq!(crate::loops::position(&g.state), before);
    assert_eq!(cx(&mut g).chars(a).keywords, Keywords::of(&[Keyword::SnowForestwalk]));
}
