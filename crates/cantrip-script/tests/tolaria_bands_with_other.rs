//! Tolaria removes every bands-with-other ability, even on a creature without ordinary banding.
use cantrip_engine::*;
use cantrip_mechanics::banding;
use std::sync::OnceLock;

fn legendary(q: &dyn Query, id: ObjectId) -> bool { q.chars(id).supertypes.legendary }
fn creature(q: &dyn Query, id: ObjectId) -> bool { q.chars(id).is(CardType::Creature) }
static SUBJECT: CardDef = CardDef {
    name: "Tolaria bands fixture",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(2),
    keywords: Keywords::of(&[Keyword::Flying]),
    abilities: &[
        banding::with_others(legendary, "Bands with other legendary creatures."),
        banding::with_others(creature, "Bands with other creatures."),
    ],
    ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&SUBJECT])))
}

#[test]
fn tolaria_removes_all_bands_with_other_but_keeps_flying() {
    let cards = table();
    let filler = cards.find("Island").unwrap();
    let mut game = Game::new(cards, &Setup { decks: vec![vec![filler; 20]; 2], ..Default::default() });
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    game.state.update_turn(|turn| { turn.number = 2; turn.active = PlayerId(1); turn.priority = PlayerId(0); turn.set_step(Step::Upkeep); });
    let quay = game.state.push_obj(Obj::new(cards.find("Tolaria").unwrap(), PlayerId(0), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, quay);
    let target = game.state.push_obj(Obj::new(cards.id_of(&SUBJECT), PlayerId(1), Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, target);
    game.state.see_statics();
    let band_count = |game: &Game| game.chars(target).abilities.iter().flatten()
        .filter(|&&a| matches!(cards.ability(a), AbilityDef::BandsWithOthers(_))).count();
    assert_eq!(band_count(&game), 2);
    assert!(!game.chars(target).has(Keyword::Banding));
    game.state.frame_push(Frame::new(FrameKind::Priority));
    let mut activated = false;
    for _ in 0..30 {
        game.run();
        if activated && game.state.stack.is_empty() {
            assert_eq!(band_count(&game), 0, "both abilities are lost");
            assert!(game.chars(target).has(Keyword::Flying), "unrelated abilities remain");
            return;
        }
        let request = game.pending().cloned().expect("priority during upkeep");
        let answer = match &request.kind {
            RequestKind::Priority { options } if !activated && request.player == PlayerId(0) => {
                let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(quay) && o.index == 1).expect("Tolaria's nonmana ability");
                let mut action = Action::of(option);
                action.targets = vec![Target::Object(target)];
                activated = true;
                Answer::Act(action)
            }
            RequestKind::Priority { .. } => Answer::Pass,
            other => panic!("unexpected request: {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("Tolaria's ability did not resolve");
}
