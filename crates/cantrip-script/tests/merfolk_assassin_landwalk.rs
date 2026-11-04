//! Merfolk Assassin's actual target predicate includes snowislandwalk (its
//! ruling), and follows text changes to the named basic land type (CR 612.2).
//! Test-only creatures keep this card independent of other unpublished cards.
use std::sync::OnceLock;
use cantrip_engine::*;

static ISLAND: CardDef = CardDef { name: "Assassin Islandwalker", types: TypeSet::of(&[CardType::Creature]), keywords: Keywords::of(&[Keyword::Islandwalk]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };
static SNOW_ISLAND: CardDef = CardDef { name: "Assassin Snow Islandwalker", keywords: Keywords::of(&[Keyword::SnowIslandwalk]), ..ISLAND };
static FOREST: CardDef = CardDef { name: "Assassin Forestwalker", keywords: Keywords::of(&[Keyword::Forestwalk]), ..ISLAND };
static SNOW_FOREST: CardDef = CardDef { name: "Assassin Snow Forestwalker", keywords: Keywords::of(&[Keyword::SnowForestwalk]), ..ISLAND };
static PLAIN: CardDef = CardDef { name: "Assassin Plain Creature", keywords: Keywords::of(&[]), ..ISLAND };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter()
        .chain([&ISLAND, &SNOW_ISLAND, &FOREST, &SNOW_FOREST, &PLAIN])))
}

fn put(g: &mut Game, card: CardId, player: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(card, player, zone));
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}

fn position() -> (Game, ObjectId, TargetFilter) {
    let mut g = Game::new(table(), &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let card = table().find("Merfolk Assassin").unwrap();
    let source = put(&mut g, card, PlayerId(0), Zone::BATTLEFIELD);
    let AbilityDef::Activated(ability) = table().get(card).abilities[0] else { panic!("actual activated ability") };
    (g, source, ability.targets[0].filter)
}

#[test]
fn actual_assassin_targets_islandwalk_and_snowislandwalk_on_either_players_creatures() {
    let (mut g, source, filter) = position();
    for player in [PlayerId(0), PlayerId(1)] {
        for (def, accepted) in [(&ISLAND, true), (&SNOW_ISLAND, true), (&FOREST, false), (&SNOW_FOREST, false), (&PLAIN, false)] {
            let target = put(&mut g, table().id_of(def), player, Zone::BATTLEFIELD);
            assert_eq!(filter(&g, &Asker::new(source, PlayerId(0)), Target::Object(target)), accepted, "{} / {player:?}", def.name);
        }
    }
}

#[test]
fn changing_island_to_forest_changes_both_the_named_walk_and_its_snow_variant() {
    let (mut g, source, filter) = position();
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect { source, controller: PlayerId(0), timestamp, affected: source,
        kind: EffectKind::ChangeText { from: Word::Subtype(Subtype::Island), to: Word::Subtype(Subtype::Forest) },
        duration: Duration::Indefinite, words: Words::NONE });
    for (def, accepted) in [(&ISLAND, false), (&SNOW_ISLAND, false), (&FOREST, true), (&SNOW_FOREST, true), (&PLAIN, false)] {
        let target = put(&mut g, table().id_of(def), PlayerId(1), Zone::BATTLEFIELD);
        assert_eq!(filter(&g, &Asker::new(source, PlayerId(0)), Target::Object(target)), accepted, "{}", def.name);
    }
}

#[test]
fn a_snowislandwalker_in_a_graveyard_is_not_a_permanent_target() {
    let (mut g, source, filter) = position();
    let target = put(&mut g, table().id_of(&SNOW_ISLAND), PlayerId(1), Zone::graveyard(PlayerId(1)));
    assert!(!filter(&g, &Asker::new(source, PlayerId(0)), Target::Object(target)));
}
