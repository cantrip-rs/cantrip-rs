//! A fork's own definitions in the engine's card table (`add_cards`): a variant's emblem, dealt to a seat's command zone
//! and found by name, and what the hook refuses. One test, since the table is the process's.

use cantrip_engine::*;

static GEAR: CardDef = CardDef { name: "Realm Test Circlet (gear)", ..CardDef::BLANK };
static TAKEN: CardDef = CardDef { name: "Lightning Bolt", ..CardDef::BLANK };
static EXTRA: [&CardDef; 1] = [&GEAR];
static CLASHING: [&CardDef; 2] = [&GEAR, &TAKEN];

#[test]
fn a_forks_definitions_join_the_table_once_before_it_is_built() {
    assert!(add_cards(&CLASHING).is_err(), "a name the engine has is refused");
    assert_eq!(add_cards(&EXTRA), Ok(()));
    assert!(add_cards(&EXTRA).is_err(), "only once");

    let emblem = emblem_named(cards(), "Realm Test Circlet (gear)").expect("the definition is in the table, an emblem");
    assert_eq!(cards().get(emblem).name, "Realm Test Circlet (gear)");
    assert!(card("Lightning Bolt").is_some(), "the engine's own cards are all still there");

    let decks = vec![vec!["Forest".to_string(); 40]; 2];
    let variant = [SeatVariantNames { emblems: vec!["Realm Test Circlet (gear)".into()], ..SeatVariantNames::default() }];
    let game = deal_with(&decks, &Outside { variant: &variant, ..Outside::default() }, 1, false, None).expect("dealt");
    let command = game.state.zone(Zone::COMMAND);
    assert_eq!(command.len(), 1);
    assert_eq!(game.state.obj(command[0]).owner, PlayerId(0));

    assert!(add_cards(&EXTRA).is_err(), "not once the table is built");
}
