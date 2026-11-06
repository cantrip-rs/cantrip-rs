//! Non-targeting attachment choices must use the same authority as attachment (CR701.3b,702.16c-d).
use crate::{Cx, Game, Setup};
use cantrip_api::*;

fn creature(q: &dyn Query, _: &Asker, target: Target) -> bool {
    target
        .object()
        .is_some_and(|id| q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}
fn your_creature(q: &dyn Query, asker: &Asker, target: Target) -> bool {
    creature(q, asker, target)
        && target
            .object()
            .is_some_and(|id| q.controller(id) == asker.controller)
}
fn land(q: &dyn Query, _: &Asker, target: Target) -> bool {
    target
        .object()
        .is_some_and(|id| q.is_on_battlefield(id) && q.chars(id).is(CardType::Land))
}
fn itself(_: &dyn LayerQuery, source: ObjectId, object: ObjectId, _: &Chars) -> bool {
    source == object
}
fn protection_white(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.protection = chars.protection.with(Color::White);
}
static PLAIN: CardDef = CardDef {
    name: "Attachment plain creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
static HEXPROOF: CardDef = CardDef {
    name: "Attachment hexproof creature",
    keywords: Keywords::of(&[Keyword::Hexproof]),
    ..PLAIN
};
static PROTECTED: CardDef = CardDef {
    name: "Attachment protected creature",
    statics: &[static_ability(
        Layer::Ability,
        itself,
        protection_white,
        "Protection from white",
    )],
    ..PLAIN
};
static LAND: CardDef = CardDef {
    name: "Attachment land",
    types: TypeSet::of(&[CardType::Land]),
    ..CardDef::BLANK
};
static AURA: CardDef = CardDef {
    name: "Attachment white Aura",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    colors: Some(ColorSet::of(Color::White)),
    enchant: Some(target(creature)),
    ..CardDef::BLANK
};
static ARTIFACT_AURA: CardDef = CardDef {
    name: "Attachment artifact Aura",
    types: TypeSet::of(&[CardType::Artifact, CardType::Enchantment]),
    ..AURA
};
static YOUR_AURA: CardDef = CardDef {
    name: "Attachment your-creature Aura",
    enchant: Some(target(your_creature)),
    ..AURA
};
static LAND_AURA: CardDef = CardDef {
    name: "Attachment land Aura",
    enchant: Some(target(land)),
    ..AURA
};
static ANIMATED_AURA: CardDef = CardDef {
    name: "Attachment creature Aura",
    types: TypeSet::of(&[CardType::Creature, CardType::Enchantment]),
    power: Some(1),
    toughness: Some(1),
    ..AURA
};
static EQUIPMENT: CardDef = CardDef {
    name: "Attachment Equipment",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    ..CardDef::BLANK
};
static AURA_EQUIPMENT: CardDef = CardDef {
    name: "Attachment Aura Equipment",
    types: TypeSet::of(&[CardType::Artifact, CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura, Subtype::Equipment]),
    ..YOUR_AURA
};
static FORTIFICATION: CardDef = CardDef {
    name: "Attachment Fortification",
    subtypes: SubtypeSet::of(&[Subtype::Fortification]),
    ..EQUIPMENT
};
static CREATURE_FORTIFICATION: CardDef = CardDef {
    name: "Attachment creature Fortification",
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..FORTIFICATION
};
static BATTLE_FORTIFICATION: CardDef = CardDef {
    name: "Attachment Battle Fortification",
    types: TypeSet::of(&[CardType::Artifact, CardType::Battle]),
    defense: Some(5),
    ..FORTIFICATION
};
static LAND_AURA_EQUIPMENT: CardDef = CardDef {
    name: "Attachment land Aura Equipment",
    enchant: Some(target(land)),
    ..AURA_EQUIPMENT
};
fn player(_: &dyn Query, _: &Asker, target: Target) -> bool {
    matches!(target, Target::Player(_))
}
static PLAYER_AURA_EQUIPMENT: CardDef = CardDef {
    name: "Attachment player Aura Equipment",
    enchant: Some(target(player)),
    ..AURA_EQUIPMENT
};
fn battlefield_artifact(q: &dyn LayerQuery, _: ObjectId, object: ObjectId, chars: &Chars) -> bool {
    q.state().obj(object).zone.is(ZoneKind::Battlefield) && chars.is(CardType::Artifact)
}
fn animate_artifact(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.types = chars.types.with(CardType::Creature);
    chars.power = Some(1);
    chars.toughness = Some(1);
}
static ARTIFACT_ANIMATOR: CardDef = CardDef {
    name: "Attachment artifact animator",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(
        Layer::Type,
        battlefield_artifact,
        animate_artifact,
        "Artifacts are creatures",
    )],
    ..CardDef::BLANK
};
fn protection_mana_value(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.protection_mana_value = Some(3);
}
static VALUE_PROTECTED: CardDef = CardDef {
    name: "Attachment mana value protected host",
    statics: &[static_ability(
        Layer::Ability,
        itself,
        protection_mana_value,
        "Protection from mana value 3 or greater",
    )],
    ..PLAIN
};
static BACK_EQUIPMENT: CardDef = CardDef {
    name: "Attachment transformed back Equipment",
    has_cost: false,
    keywords: Keywords::of(&[Keyword::Nightbound]),
    ..EQUIPMENT
};
static FRONT_EQUIPMENT: CardDef = CardDef {
    name: "Attachment front Equipment",
    cost: ManaCost {
        generic: 3,
        ..ManaCost::ZERO
    },
    has_cost: true,
    back: Some(&BACK_EQUIPMENT),
    keywords: Keywords::of(&[Keyword::Daybound]),
    ..EQUIPMENT
};
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &PLAIN,
        &HEXPROOF,
        &PROTECTED,
        &LAND,
        &AURA,
        &ARTIFACT_AURA,
        &YOUR_AURA,
        &LAND_AURA,
        &ANIMATED_AURA,
        &EQUIPMENT,
        &AURA_EQUIPMENT,
        &FORTIFICATION,
        &CREATURE_FORTIFICATION,
        &BATTLE_FORTIFICATION,
        &LAND_AURA_EQUIPMENT,
        &PLAYER_AURA_EQUIPMENT,
        &ARTIFACT_ANIMATOR,
        &VALUE_PROTECTED,
        &FRONT_EQUIPMENT,
        &BACK_EQUIPMENT,
    ])));
    Game::new(
        cards,
        &Setup {
            decks: vec![vec![], vec![]],
            ..Default::default()
        },
    )
}
fn put(g: &mut Game, card: &'static CardDef, player: u8) -> ObjectId {
    let mut object = Obj::new(g.cards.id_of(card), PlayerId(player), Zone::BATTLEFIELD);
    object.timestamp = g.state.timestamp();
    let id = g.state.push_obj(object);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    if !card.statics.is_empty() {
        g.state.see_statics();
    }
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: usize::MAX,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: vec![],
        damage: None,
        asked_at: None,
        doer: None,
        producing: None,
    }
}
#[test]
fn untargeted_hosts_ignore_hexproof_but_obey_protection_and_match_actual_attachment() {
    let mut g = game();
    let aura = put(&mut g, &AURA, 0);
    let protected = put(&mut g, &PROTECTED, 1);
    let hexproof = put(&mut g, &HEXPROOF, 1);
    let asker = Asker::new(aura, PlayerId(0));
    assert!(!g.can_target(&asker, Target::Object(hexproof)));
    assert!(g.can_attach_to(aura, hexproof));
    assert!(!g.can_attach_to(aura, protected));
    assert!(!cantrip_api::Game::attach(&mut cx(&mut g), aura, protected));
    assert!(g.state.obj(aura).attached_to.is_none());
    assert!(cx(&mut g).can_attach_to(aura, hexproof));
    assert!(cantrip_api::Game::attach(&mut cx(&mut g), aura, hexproof));
    assert_eq!(g.state.obj(aura).attached_to, Some(hexproof));
    // Host legality alone does not impose a caller's separate "another" restriction.
    assert!(g.can_attach_to(aura, hexproof));
}
#[test]
fn enchant_restrictions_use_the_auras_controller_and_current_host_type() {
    let mut g = game();
    let aura = put(&mut g, &YOUR_AURA, 0);
    let own = put(&mut g, &PLAIN, 0);
    let other = put(&mut g, &PLAIN, 1);
    let land = put(&mut g, &LAND, 0);
    let land_aura = put(&mut g, &LAND_AURA, 1);
    assert!(g.can_attach_to(aura, own));
    assert!(!g.can_attach_to(aura, other));
    assert!(!g.can_attach_to(aura, land));
    assert!(g.can_attach_to(land_aura, land));
    assert!(!g.can_attach_to(land_aura, own));
    g.state.update_obj(aura, |o| o.controller = PlayerId(1));
    assert!(!g.can_attach_to(aura, own));
    assert!(g.can_attach_to(aura, other));
}
#[test]
fn creature_auras_equipment_and_old_incarnations_follow_the_core_host_rules() {
    let mut g = game();
    let animated = put(&mut g, &ANIMATED_AURA, 0);
    let aura = put(&mut g, &AURA, 0);
    let equipment = put(&mut g, &EQUIPMENT, 0);
    let host = put(&mut g, &PLAIN, 0);
    let land = put(&mut g, &LAND, 0);
    assert!(!g.can_attach_to(animated, host));
    assert!(!g.can_attach_to(aura, aura));
    assert!(g.can_attach_to(equipment, host));
    assert!(!g.can_attach_to(equipment, land));
    assert!(!g.can_attach_to(host, land));
    g.state.update_obj(host, |o| o.became = Some(land));
    assert!(!g.can_attach_to(aura, host));
    assert!(!g.can_attach_to(equipment, host));
}

#[test]
fn equipment_that_is_also_an_aura_obeys_both_attachment_restrictions() {
    let mut g = game();
    let aura = put(&mut g, &AURA_EQUIPMENT, 0);
    let own = put(&mut g, &PLAIN, 0);
    let other = put(&mut g, &PLAIN, 1);
    let land = put(&mut g, &LAND, 0);
    assert!(g.can_attach_to(aura, own));
    assert!(
        !g.can_attach_to(aura, other),
        "Equipment does not override enchant creature you control"
    );
    assert!(!g.can_attach_to(aura, land));
    assert!(cantrip_api::Game::attach(&mut cx(&mut g), aura, own));
    assert!(!cantrip_api::Game::attach(&mut cx(&mut g), aura, other));
    assert_eq!(g.state.obj(aura).attached_to, Some(own));
}

#[test]
fn fortifications_attach_to_lands_and_creature_fortifications_cannot() {
    let mut g = game();
    let fortification = put(&mut g, &FORTIFICATION, 0);
    let animated = put(&mut g, &CREATURE_FORTIFICATION, 0);
    let land = put(&mut g, &LAND, 1);
    let creature = put(&mut g, &PLAIN, 1);
    assert!(g.can_attach_to(fortification, land));
    assert!(!g.can_attach_to(fortification, creature));
    assert!(!g.can_attach_to(animated, land));
    assert!(cantrip_api::Game::attach(
        &mut cx(&mut g),
        fortification,
        land
    ));
    assert_eq!(g.state.obj(fortification).attached_to, Some(land));
}

#[test]
fn state_based_actions_keep_a_legal_fortification_attached() {
    let mut g = game();
    let fortification = put(&mut g, &FORTIFICATION, 0);
    let land = put(&mut g, &LAND, 1);
    g.state
        .update_obj(fortification, |o| o.attached_to = Some(land));
    crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap();
    assert_eq!(g.state.obj(fortification).attached_to, Some(land));
}

#[test]
fn a_battle_cannot_be_attached_even_if_it_has_an_attachment_subtype() {
    let mut g = game();
    let attachment = put(&mut g, &BATTLE_FORTIFICATION, 0);
    let land = put(&mut g, &LAND, 0);
    assert!(!g.can_attach_to(attachment, land));
    assert!(!cantrip_api::Game::attach(
        &mut cx(&mut g),
        attachment,
        land
    ));
}

#[test]
fn incoming_aura_host_choices_obey_equipment_restrictions_too() {
    let mut g = game();
    let attachment = put(&mut g, &LAND_AURA_EQUIPMENT, 0);
    let land = put(&mut g, &LAND, 0);
    let chars = g.chars(attachment);
    let mana_value = g.mana_value(attachment);
    let hosts = crate::attach::aura_hosts(
        &cx(&mut g),
        attachment,
        &chars,
        PlayerId(0),
        mana_value,
        &[attachment],
    );
    assert!(
        !hosts.contains(&Target::Object(land)),
        "enchant land cannot override Equipment's creature restriction"
    );
}

#[test]
fn an_aura_equipment_cannot_enchant_a_player() {
    let mut g = game();
    let attachment = put(&mut g, &PLAYER_AURA_EQUIPMENT, 0);
    assert!(!crate::attach::player_allows(
        &cx(&mut g),
        attachment,
        PlayerId(1)
    ));
    let chars = g.chars(attachment);
    let mana_value = g.mana_value(attachment);
    let hosts = crate::attach::aura_hosts(
        &cx(&mut g),
        attachment,
        &chars,
        PlayerId(0),
        mana_value,
        &[attachment],
    );
    assert!(!hosts.contains(&Target::Player(PlayerId(1))));
}

#[test]
fn an_incoming_equipment_is_judged_as_it_would_exist_on_the_battlefield() {
    for animated in [false, true] {
        let mut g = game();
        if animated {
            put(&mut g, &ARTIFACT_ANIMATOR, 0);
        }
        let host = put(&mut g, &PLAIN, 0);
        let hand = Zone::hand(PlayerId(0));
        let id = g
            .state
            .push_obj(Obj::new(g.cards.id_of(&EQUIPMENT), PlayerId(0), hand));
        g.state.zone_push(hand, id);
        assert!(
            !g.chars(id).is(CardType::Creature),
            "its hand characteristics are not prospective"
        );
        let entered =
            crate::ops::put_onto_battlefield_attached(&mut cx(&mut g), id, PlayerId(0), host)
                .unwrap()
                .unwrap();
        assert_eq!(g.chars(entered).is(CardType::Creature), animated);
        assert_eq!(
            g.state.obj(entered).attached_to,
            if animated { None } else { Some(host) }
        );
        assert_eq!(g.log.iter().any(|record| matches!(record.event,
            Event::Attached { attachment, to: Target::Object(to) } if attachment == entered && to == host
        )), !animated, "an illegal attachment must never be emitted, even if an SBA would detach it later");
    }
}

pub(crate) fn check_projected_face_attachment(
    check: impl FnOnce(&Cx<'_>, ObjectId, ObjectId, CardId) -> (u32, bool),
) {
    let mut g = game();
    let host = put(&mut g, &VALUE_PROTECTED, 0);
    let hand = Zone::hand(PlayerId(0));
    let attachment = g
        .state
        .push_obj(Obj::new(g.cards.id_of(&FRONT_EQUIPMENT), PlayerId(0), hand));
    g.state.zone_push(hand, attachment);
    let face = g.cards.id_of(&BACK_EQUIPMENT);
    let (mana_value, allowed) = check(&cx(&mut g), attachment, host, face);
    assert_eq!(
        mana_value, 3,
        "a costless nonmodal back face uses its front's mana value"
    );
    assert!(
        !allowed,
        "host legality must preserve a Query view's mana-value override"
    );
}

#[test]
fn an_incoming_transformed_attachment_has_its_front_faces_mana_value() {
    let mut g = game();
    g.state.update_turn(|turn| turn.day_night = DayNight::Night);
    let host = put(&mut g, &VALUE_PROTECTED, 0);
    let hand = Zone::hand(PlayerId(0));
    let attachment = g
        .state
        .push_obj(Obj::new(g.cards.id_of(&FRONT_EQUIPMENT), PlayerId(0), hand));
    g.state.zone_push(hand, attachment);
    let entered =
        crate::ops::put_onto_battlefield_attached(&mut cx(&mut g), attachment, PlayerId(0), host)
            .unwrap()
            .unwrap();
    assert!(g.state.obj(entered).transformed);
    assert_eq!(g.mana_value(entered), 3);
    assert_eq!(g.state.obj(entered).attached_to, None);
    assert!(
        !g.log.iter().any(|record| matches!(record.event,
            Event::Attached { attachment, .. } if attachment == entered
        )),
        "protection must refuse the attachment before entry, not detach it later"
    );
}

#[test]
fn equipment_token_uses_prospective_battlefield_characteristics() {
    for animated in [false, true] {
        let mut g = game();
        if animated { put(&mut g, &ARTIFACT_ANIMATOR, 0); }
        let host = put(&mut g, &PLAIN, 0);
        let card = g.cards.id_of(&EQUIPMENT);
        let created = crate::ops::create_token_attached(&mut cx(&mut g), card, PlayerId(0), host).unwrap();
        assert_eq!(created.len(), 1);
        let token = created[0];
        assert_eq!(g.chars(token).is(CardType::Creature), animated);
        assert_eq!(g.state.obj(token).attached_to, if animated { None } else { Some(host) });
        assert_eq!(g.log.iter().any(|record| matches!(record.event,
            Event::Attached { attachment, to: Target::Object(to) } if attachment == token && to == host
        )), !animated, "illegal token attachment must not occur even transiently");
    }
}

#[test]
fn aura_token_that_would_be_a_creature_is_not_created() {
    for animated in [false, true] {
        let mut g = game();
        if animated { put(&mut g, &ARTIFACT_ANIMATOR, 0); }
        let host = put(&mut g, &PLAIN, 0);
        let card = g.cards.id_of(&ARTIFACT_AURA);
        let created = crate::ops::create_token_attached(&mut cx(&mut g), card, PlayerId(0), host).unwrap();
        if animated {
            assert!(created.is_empty(), "an Aura creature cannot enchant and is not created");
        } else {
            assert_eq!(created.len(), 1);
            assert_eq!(g.state.obj(created[0]).attached_to, Some(host));
        }
    }
}

#[test]
fn transformed_equipment_token_uses_prospective_mana_value_for_protection() {
    for protected in [false, true] {
        let mut g = game();
        g.state.update_turn(|turn| turn.day_night = DayNight::Night);
        let host = put(&mut g, if protected { &VALUE_PROTECTED } else { &PLAIN }, 0);
        let card = g.cards.id_of(&FRONT_EQUIPMENT);
        let created = crate::ops::create_token_attached(&mut cx(&mut g), card, PlayerId(0), host).unwrap();
        assert_eq!(created.len(), 1);
        let token = created[0];
        assert!(g.state.obj(token).transformed);
        assert_eq!(g.mana_value(token), 3);
        assert_eq!(g.state.obj(token).attached_to, if protected { None } else { Some(host) });
        assert_eq!(g.log.iter().any(|r| matches!(r.event,
            Event::Attached { attachment, to: Target::Object(to) } if attachment == token && to == host
        )), !protected);
    }
}
