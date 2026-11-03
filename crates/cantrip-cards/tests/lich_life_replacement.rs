use cantrip_api::*;

// The predicate must reject a zero proposal left by an earlier replacement, so it cannot
// create another replacement choice or draw instruction for an event that no longer exists.
struct UnusedQuery;
impl Query for UnusedQuery {
    fn can_attach_to(&self, _: ObjectId, _: ObjectId) -> bool { unreachable!() }
        fn can_untap(&self, _: ObjectId) -> bool { unreachable!() }
    fn state(&self) -> &GameState { panic!("predicate does not inspect state") }
    fn card(&self, _: CardId) -> &'static CardDef { panic!("unused") }
    fn chars(&self, _: ObjectId) -> Chars { panic!("unused") }
    fn token_chars(&self, _: CardId, _: Option<Copiable>) -> Chars { panic!("unused") }
    fn lost_all_abilities(&self, _: ObjectId) -> bool { panic!("unused") }
    fn card_id(&self, _: &'static CardDef) -> CardId { panic!("unused") }
    fn events(&self) -> &[Record] { panic!("unused") }
    fn linked_card(&self, _: CardId) -> CardId { panic!("unused") }
    fn names_its_source(&self, _: Waiting) -> bool { panic!("unused") }
    fn text_linked(&self, _: ObjectId) -> Linked { panic!("unused") }
    fn can_target(&self, _: &Asker, _: Target) -> bool { panic!("unused") }
}

#[test]
fn only_positive_life_gains_of_the_controller_are_replaced() {
    let card = cantrip_cards::all().into_iter().find(|card| card.name == "Lich").unwrap();
    let applies = card.replacements[0].applies;
    let me = PlayerId(0);
    let asker = Asker::new(ObjectId(0), me);
    let event = |player, amount| Proposal::LifeChange(LifeChange { player, amount });
    assert!(applies(&UnusedQuery, &asker, &event(me, 1)));
    assert!(!applies(&UnusedQuery, &asker, &event(me, 0)));
    assert!(!applies(&UnusedQuery, &asker, &event(me, -1)));
    assert!(!applies(&UnusedQuery, &asker, &event(PlayerId(1), 1)));
}
