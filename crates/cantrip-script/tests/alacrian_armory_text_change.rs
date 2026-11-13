//! Alacrian Armory's "target Mount or Vehicle you control" reads the creature type Mount and the artifact type Vehicle
//! through its own words, so a text-changing effect that turns its "Mount" into "Dinosaur" makes it target Dinosaurs and
//! not Mounts (CR 612.2, 612.1). Test-only permanents keep this card independent of other unpublished cards.
use std::sync::OnceLock;
use cantrip_engine::*;

static MOUNT: CardDef = CardDef {
    name: "Armory Test Mount",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Horse, Subtype::Mount]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
static DINOSAUR: CardDef = CardDef { name: "Armory Test Dinosaur", subtypes: SubtypeSet::of(&[Subtype::Dinosaur]), ..MOUNT };
static VEHICLE: CardDef = CardDef {
    name: "Armory Test Vehicle",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Vehicle]),
    ..MOUNT
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MOUNT, &DINOSAUR, &VEHICLE])))
}

fn put(g: &mut Game, card: CardId, player: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(card, player, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

fn position() -> (Game, ObjectId, TargetFilter) {
    let mut g = Game::new(table(), &Setup { decks: vec![Vec::new(); 2], ..Default::default() });
    let card = table().find("Alacrian Armory").unwrap();
    let source = put(&mut g, card, PlayerId(0));
    let AbilityDef::Triggered(ability) = table().get(card).abilities[0] else { panic!("printed triggered ability") };
    (g, source, ability.targets[0].filter)
}

#[test]
fn armory_targets_mounts_and_vehicles_you_control_only() {
    let (mut g, source, filter) = position();
    for (player, mine) in [(PlayerId(0), true), (PlayerId(1), false)] {
        for (def, accepted) in [(&MOUNT, true), (&VEHICLE, true), (&DINOSAUR, false)] {
            let target = put(&mut g, table().id_of(def), player);
            assert_eq!(filter(&g, &Asker::new(source, PlayerId(0)), Target::Object(target)), accepted && mine, "{} / {player:?}", def.name);
        }
    }
}

#[test]
fn changing_mount_to_dinosaur_makes_the_armory_target_dinosaurs_instead() {
    let (mut g, source, filter) = position();
    let timestamp = g.state.timestamp();
    g.state.effect_push(ContinuousEffect { source, controller: PlayerId(0), timestamp, affected: source,
        kind: EffectKind::ChangeText { from: Word::Subtype(Subtype::Mount), to: Word::Subtype(Subtype::Dinosaur) },
        duration: Duration::Indefinite, words: Words::NONE });
    for (def, accepted) in [(&MOUNT, false), (&VEHICLE, true), (&DINOSAUR, true)] {
        let target = put(&mut g, table().id_of(def), PlayerId(0));
        assert_eq!(filter(&g, &Asker::new(source, PlayerId(0)), Target::Object(target)), accepted, "{}", def.name);
    }
}
