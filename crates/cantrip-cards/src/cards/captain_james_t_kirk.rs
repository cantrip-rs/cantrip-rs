use cantrip_api::*;
use cantrip_mechanics::effects::discard_then_draw;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

static MODES: [ModeDef; 3] = [
    mode(
        |g, item| {
            discard_then_draw(g, item, 1)?;
            Ok(())
        },
        "Discard a card, then draw a card.",
    ),
    mode(
        |g, item| {
            tokens::create(g, item.controller, &tokens::OFFICER, 1)?;
            Ok(())
        },
        "Create a 1/1 red Officer creature token.",
    ),
    mode(
        effects::creatures_you_control_get::<1, 0>,
        "Creatures you control get +1/+0 until end of turn.",
    ),
];

static MODAL: ModalDef = modes(&MODES).up_to_if(hand_is_empty, 3);

fn hand_is_empty(q: &dyn Query, asker: &Asker) -> bool {
    q.state().zone(Zone::hand(asker.controller)).is_empty()
}

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        &MODAL,
        "Whenever Captain Kirk enters or attacks, choose one. If you have no cards in hand, choose one or more instead.",
    )],
    ..oracle::CAPTAIN_JAMES_T_KIRK
};

pub static EXTRA: &[&CardDef] = &[&tokens::OFFICER];
