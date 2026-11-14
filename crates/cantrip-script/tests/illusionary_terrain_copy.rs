//! Copying an enchantment copies its text, not its already-made choices (CR 707.2, 614.12a).
use cantrip_engine::*;
use cantrip_mechanics::{copy, targets};
use cantrip_script::{GameScript, run_on};

static COPIER: CardDef = CardDef {
    name: "Probe Enchantment Copier",
    oracle: true,
    has_cost: true,
    cost: mana("{0}"),
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{0}"), copy::create_token_copy_of_target, "{0}: Create a token copy of target enchantment.").targets(&[targets::ENCHANTMENT])],
    ..CardDef::BLANK
};

#[test]
fn a_token_copy_makes_its_own_ordered_choices() {
    let source = "
[Bot1 Deck]
Forest
Mox Sapphire x2
Illusionary Terrain
Probe Enchantment Copier
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Forest.Play
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Probe Enchantment Copier.Cast
Bot1.Battlefield.Mox Sapphire#1.Activate
Bot1.Battlefield.Mox Sapphire#2.Activate
Bot1.Hand.Illusionary Terrain.Cast
Bot1.Modes(Forest)
Bot1.Modes(Island)
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Probe Enchantment Copier.Activate(Illusionary Terrain)
Bot1.Pass
Bot2.Pass
Bot1.Modes(Island)
Bot1.Modes(Swamp)
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Count = 6
Bot1.Battlefield.Forest.Subtypes = Swamp
";
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&COPIER]))));
    let script = GameScript::parse(source, "illusionary-copy").unwrap();
    let game = run_on(&script, table).map_err(|f| f.report).unwrap().game;
    let terrains: Vec<_> = game.state.battlefield().iter().copied().filter(|&id| game.cards.get(game.state.obj(id).card).name == "Illusionary Terrain").collect();
    assert_eq!(terrains.len(), 2);
    for id in terrains {
        let chosen = game.state.chosen_subtype_pair(game.cards.linked_to(&game.state, id));
        if game.state.obj(id).token {
            assert_eq!(chosen, Some([Subtype::Island, Subtype::Swamp]));
        } else {
            assert_eq!(chosen, Some([Subtype::Forest, Subtype::Island]));
        }
    }
}

#[test]
fn chosen_pair_remains_linked_when_the_terrain_loses_and_regains_its_abilities() {
    let source = "
[Bot1 Deck]
Forest
Mox Sapphire x2
Illusionary Terrain
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Forest.Play
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Battlefield.Mox Sapphire#1.Activate
Bot1.Battlefield.Mox Sapphire#2.Activate
Bot1.Hand.Illusionary Terrain.Cast
Bot1.Modes(Forest)
Bot1.Modes(Island)
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Battlefield.Forest.Subtypes = Island
";
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let script = GameScript::parse(source, "illusionary-lose-regain").unwrap();
    let mut game = run_on(&script, table).map_err(|f| f.report).unwrap().game;
    let find = |name| game.state.battlefield().iter().copied().find(|&id| game.cards.get(game.state.obj(id).card).name == name).unwrap();
    let terrain = find("Illusionary Terrain");
    let forest = find("Forest");
    let by = game.cards.linked_to(&game.state, terrain);
    let timestamp = game.state.next_timestamp;
    game.state.effect_push(ContinuousEffect { source: terrain, controller: PlayerId(0), timestamp, affected: terrain,
        kind: EffectKind::LoseAllAbilities, duration: Duration::Indefinite, words: Words::NONE });
    // The type-changing ability applies in layer 4 before ability removal in layer 6 (CR 613.1d,f).
    assert!(game.chars(terrain).lost_all);
    assert!(game.chars(forest).has_subtype(Subtype::Island));
    assert_eq!(game.state.chosen_subtype_pair(by), Some([Subtype::Forest, Subtype::Island]));
    game.state.effects_retain(|e| e.timestamp != timestamp);
    assert!(game.chars(forest).has_subtype(Subtype::Island));
    assert_eq!(game.state.chosen_subtype_pair(by), Some([Subtype::Forest, Subtype::Island]));

}

fn copy_away_until_end_of_turn(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(target) = item.target(0).object() {
        g.become_copy(item.object, target, Copiable::of(g.card_id(&COPIER)), Duration::EndOfTurn);
    }
    Ok(())
}
static COPY_AWAY: CardDef = CardDef {
    name: "Probe Temporary Copy",
    oracle: true,
    has_cost: true,
    cost: mana("{0}"),
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{0}"), copy_away_until_end_of_turn, "{0}: Target enchantment becomes a copy of Probe Enchantment Copier until end of turn.").targets(&[targets::ENCHANTMENT])],
    ..CardDef::BLANK
};

#[test]
fn copying_away_and_back_restores_the_original_entry_choices() {
    let source = "
[Bot1 Deck]
Forest
Mox Sapphire x2
Illusionary Terrain
Probe Temporary Copy
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Forest.Play
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Probe Temporary Copy.Cast
Bot1.Battlefield.Mox Sapphire#1.Activate
Bot1.Battlefield.Mox Sapphire#2.Activate
Bot1.Hand.Illusionary Terrain.Cast
Bot1.Modes(Forest)
Bot1.Modes(Island)
Bot1.Pass
Bot2.Pass
Bot1.Battlefield.Probe Temporary Copy.Activate(Illusionary Terrain)
Bot1.Pass
Bot2.Pass
? Bot1.Battlefield.Forest.Subtypes = Forest
[T2 Bot2.Main1]
[Expect]
Bot1.Battlefield.Forest.Subtypes = Island
Bot1.Battlefield.Illusionary Terrain.FirstChosenSubtype = Forest
Bot1.Battlefield.Illusionary Terrain.SecondChosenSubtype = Island
";
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&COPIER, &COPY_AWAY]))));
    let script = GameScript::parse(source, "illusionary-copy-away").unwrap();
    run_on(&script, table).map_err(|f| f.report).unwrap();
}
