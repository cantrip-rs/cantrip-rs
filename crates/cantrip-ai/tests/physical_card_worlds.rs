//! A merged permanent represents several deck cards (CR 730.2-3). Dealing hidden
//! worlds must retain each card's origin both while merged and after separation.
use cantrip_ai::{Knowledge, Prior, determinize, observe_indexed};
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_script::GameScript;

const CASE: &str = include_str!(
    "../../../scenarios/cling-to-dust-escapes-exiling-both-cards-of-a-merged-creature-ashnods-altar-sacrificed.cantrip"
);

fn position(on_top: bool, separated: bool) -> Game {
    let stop = if separated {
        "[T5 Bot1.Main2]"
    } else {
        "[T3 Bot1.Main1]"
    };
    let mut text = format!(
        "{}\n{stop}\n[Handover Bot1.Pass]\n",
        CASE.split(stop).next().unwrap()
    );
    if on_top {
        text = text
            .replace("Bot1.Modes(2)", "Bot1.Modes(1)")
            .replace("Sacrifice: Ornithopter", "Sacrifice: Cubwarden");
    }
    let script = GameScript::parse(&text, "physical-card-world").unwrap();
    let result =
        cantrip_script::run(&script).unwrap_or_else(|f| panic!("{}\n{}", f.message, f.report));
    assert!(result.handed_over);
    let game = result.game;
    let active = game
        .state
        .battlefield()
        .iter()
        .filter(|&&id| game.state.merge_of(id).is_some())
        .count();
    assert_eq!(active, usize::from(!separated));
    game
}

fn check(game: &Game) {
    for seat in [PlayerId(0), PlayerId(1)] {
        for mode in [InfoMode::Decklist, InfoMode::Hidden] {
            let knowledge = Knowledge::of(game, seat);
            let before = observe_indexed(game, &knowledge, seat, mode)
                .map(|(o, _)| cantrip_obs::to_bytes(&o));
            assert!(before.is_some());
            let mut world = game.fork_for_search();
            world.reseed(5);
            determinize(
                &mut world,
                &knowledge,
                seat,
                mode,
                &Prior::new(),
                &mut Rng::new(5),
            );
            let after = observe_indexed(&world, &knowledge, seat, mode)
                .map(|(o, _)| cantrip_obs::to_bytes(&o));
            assert_eq!(
                before, after,
                "seat {} {mode:?} sees a different game",
                seat.0
            );
        }
    }
}

#[test]
fn both_cards_are_accounted_for_with_a_card_mutated_under() {
    check(&position(false, false));
}

#[test]
fn both_cards_are_accounted_for_with_a_card_mutated_over() {
    check(&position(true, false));
}

#[test]
fn both_cards_are_accounted_for_after_a_merge_with_the_card_under_separates() {
    check(&position(false, true));
}

#[test]
fn both_cards_are_accounted_for_after_a_merge_with_the_card_over_separates() {
    check(&position(true, true));
}

fn second_merge(onto_token: bool, on_top: bool, separated: bool) -> Game {
    let prefix = CASE.split("[T3 Bot1.Main1]").next().unwrap().replace(
        "Cling to Dust\nLotus Petal\nLotus Petal\n",
        "Cubwarden\nBlack Lotus\nLotus Petal\n",
    );
    let target = if onto_token {
        "Cat Token#1"
    } else {
        "Ornithopter"
    };
    let modes = if on_top { 1 } else { 2 };
    let mut text = format!(
        "{prefix}
[T3 Bot1.Main1]
Bot1.Battlefield.Plains.Activate
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Ashnod's Altar.Cast
[T5 Bot1.Main1]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Battlefield.Mox Pearl.Activate
Bot1.Hand.Cubwarden.Cast({target}; Ability: Mutate)
Bot1.Pass
Bot1.Modes({modes})
Bot1.Pass
"
    );
    if separated {
        let creature = if on_top { "Cubwarden" } else { target };
        text.push_str(&format!(
            "Bot1.Battlefield.Ashnod's Altar.Activate(Sacrifice: {creature})\n"
        ));
    }
    text.push_str("[Handover Bot1.Pass]\n");
    let script = GameScript::parse(&text, "second-merge-world").unwrap();
    let result =
        cantrip_script::run(&script).unwrap_or_else(|f| panic!("{}\n{}", f.message, f.report));
    assert!(result.handed_over);
    let game = result.game;
    let active = game
        .state
        .battlefield()
        .iter()
        .filter(|&&id| game.state.merge_of(id).is_some())
        .count();
    assert_eq!(active, usize::from(onto_token) + usize::from(!separated));
    game
}

#[test]
fn repeated_mutations_and_token_components_keep_their_physical_card_counts() {
    for onto_token in [false, true] {
        for on_top in [false, true] {
            check(&second_merge(onto_token, on_top, false));
        }
    }
}

#[test]
fn repeated_mutations_and_token_components_keep_their_origins_after_separation() {
    for onto_token in [false, true] {
        for on_top in [false, true] {
            check(&second_merge(onto_token, on_top, true));
        }
    }
}
