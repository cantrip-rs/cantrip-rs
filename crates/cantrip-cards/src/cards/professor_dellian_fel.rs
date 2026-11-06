use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_GAIN_LIFE;
use cantrip_mechanics::{destroy_target, effects, targets, tokens};
use cantrip_oracle as oracle;

/// +2: "You gain 3 life." (CR 119.3)
fn gain_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 3)
}

/// −3: "Destroy target creature." (CR 701.8)
fn destroy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// −6: "You get an emblem with ..." (CR 114.2)
fn emblem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

// --- The emblem ---------------------------------------------------------------------------------------------------

/// "Whenever you gain life, target opponent loses that much life": the amount is the life that event gained (the
/// trigger's data, CR 603.10: it is what was gained when the ability triggered). With its target gone it does nothing
/// (CR 608.2b).
fn target_opponent_loses_that_much(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(opponent) = targets::player(g, item, 0) {
        g.lose_life(opponent,(item.data[0] as i32) as i32);
    }
    Ok(())
}

pub static EMBLEM: CardDef = CardDef {
    name: "Professor Dellian Fel emblem",
    has_cost: false,
    abilities: &[triggered(YOU_GAIN_LIFE, target_opponent_loses_that_much, "Whenever you gain life, target opponent loses that much life.")
        .targets(&[targets::OPPONENT])],
    text: "Whenever you gain life, target opponent loses that much life.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(2), gain_three, "+2: You gain 3 life."),
        ability(loyalty(0), effects::draw_a_card_and_lose_1_life, "0: You draw a card and lose 1 life."),
        ability(loyalty(-3), destroy, "\u{2212}3: Destroy target creature.").targets(&[targets::CREATURE]),
        ability(loyalty(-6), emblem, "\u{2212}6: You get an emblem with \"Whenever you gain life, target opponent loses that much life.\""),
    ],
    ..oracle::PROFESSOR_DELLIAN_FEL
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];
