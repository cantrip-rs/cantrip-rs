//! A real Song and Goblin Welder, with test-only Coating and Coldsteel Heart entry shapes (CR 614.12, 611.3a).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_api::Game as Rules;
use cantrip_script::{GameScript, run_on};

fn any_permanent(q: &dyn Query, _: &Asker, target: Target) -> bool {
    matches!(target, Target::Object(id) if q.is_on_battlefield(id))
}
fn coat(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.add_effect(item.object, id, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact])), Duration::EndOfTurn);
    }
    Ok(())
}
// An artifact creature preserves Liquimetal Coating's ability under Song so it can apply the same printed effect.
static COATING: CardDef = CardDef {
    name: "Song Test Coating", oracle: true, has_cost: true, types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1), toughness: Some(1), abilities: &[ability(TAP, coat, "{T}: Target permanent becomes an artifact until end of turn.").targets(&[target(any_permanent)])],
    ..CardDef::BLANK
};
// Coldsteel Heart's relevant two entry instructions; no mana choice is needed if Song removes both.
fn heart_entry(g: &mut dyn Rules, e: &mut Entering) -> Result<(), Suspend> {
    e.tapped = true;
    e.chosen_color = Some(cantrip_mechanics::choose_color(g, e.controller, e.object)?);
    Ok(())
}
static HEART: CardDef = CardDef {
    name: "Song Test Coldsteel Heart", oracle: true, has_cost: true, cost: ManaCost { generic: 2, ..ManaCost::ZERO },
    types: TypeSet::of(&[CardType::Artifact]), text: "This artifact enters tapped. As it enters, choose a color.",
    enters: Some(heart_entry), ..CardDef::BLANK
};
fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&COATING, &HEART])))
}

#[test]
fn welder_exchange_projects_the_departing_songs_effect_onto_the_incoming_artifact() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus x2
Mox Ruby
Goblin Welder
Entomb
Titania's Song
Song Test Coating
Song Test Coldsteel Heart
Forest x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Black Lotus#1.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Ruby.Cast
Bot1.Hand.Song Test Coating.Cast
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Goblin Welder.Cast
Bot1.Battlefield.Black Lotus#1.Activate(Color: Black)
Bot1.Hand.Entomb.Cast
Bot1.Search(Song Test Coldsteel Heart)
[T3 Bot1.Main1]
Bot1.Hand.Forest.Play
Bot1.Battlefield.Black Lotus.Activate(Color: Green)
Bot1.Battlefield.Forest.Activate
Bot1.Hand.Titania's Song.Cast
Bot1.Battlefield.Song Test Coating.Activate(Bot1.Battlefield.Titania's Song)
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Goblin Welder.Activate
Bot1.Target(Titania's Song)
Bot1.Target(Song Test Coldsteel Heart)
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield.Song Test Coldsteel Heart.Tapped = false
[Expect]
Bot1.Battlefield.Song Test Coldsteel Heart.Types = Artifact; Creature
Bot1.Battlefield.Song Test Coldsteel Heart.Toughness = 2
"#, "song simultaneous entry").unwrap();
    let result = run_on(&script, table()).map_err(|e| e.report).unwrap();
    let game = result.game;
    let heart = game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).card == table().id_of(&HEART)).unwrap();
    assert!(!game.state.obj(heart).tapped);
    assert_eq!(game.state.chosen_color(table().linked_to(&game.state, heart)), None, "the entering color choice is removed too");
}

#[test]
fn coldsteel_fixture_enters_tapped_and_chooses_without_song() {
    let script = GameScript::parse(r#"
[Bot1 Deck]
Black Lotus
Song Test Coldsteel Heart
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Song Test Coldsteel Heart.Cast
Bot1.Colors(Red)
[Expect]
Bot1.Battlefield.Song Test Coldsteel Heart.Tapped = true
"#, "coldsteel entry control").unwrap();
    let result = run_on(&script, table()).map_err(|e| e.report).unwrap();
    let game = result.game;
    let heart = game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).card == table().id_of(&HEART)).unwrap();
    assert_eq!(game.state.chosen_color(table().linked_to(&game.state, heart)), Some(Color::Red));
}

#[test]
fn two_printed_songs_retain_separate_effects_when_they_leave_together() {
    let script = GameScript::parse(include_str!("../../../scenarios/titanias-song-two-effects-survive-together.cantrip"), "two songs").unwrap();
    let result = run_on(&script, table()).map_err(|e| e.report).unwrap();
    let retained: Vec<_> = result.game.state.effects.iter().filter(|effect| matches!(effect.kind, EffectKind::RetainedStatic(_))).collect();
    assert_eq!(retained.len(), 6, "both Songs retain their three layer parts");
    let sources: std::collections::HashSet<_> = retained.iter().map(|effect| effect.source).collect();
    assert_eq!(sources.len(), 2, "the two sources keep separate effect identities");
}
