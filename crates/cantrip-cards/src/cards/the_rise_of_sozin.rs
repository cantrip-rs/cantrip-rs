use cantrip_api::*;
use cantrip_mechanics::library::exile_up_to_four_named_from_graveyard_hand_and_library;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT;
use cantrip_mechanics::{effects, exile_and_return_transformed, firebending, select, targets};
use cantrip_oracle as oracle;

/// III: "Exile this Saga, then return it to the battlefield transformed under your control": the ability's controller's,
/// who need not own it (CR 712.14a: a Saga returned transformed is Fire Lord Sozin).
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_and_return_transformed(g, item.object, item.controller)
}

// --- Fire Lord Sozin ---------------------------------------------------------------------------------------------

const PAY_X: Question = Question::new(
    "PutCreaturesFromGraveyard",
    "pay {X}?",
    "When you do, put any number of target creature cards with total mana value X or less from that player's graveyard onto the battlefield under your control",
    "Pay {cost}",
    "Don't pay",
)
.no_default();

/// "Whenever Fire Lord Sozin deals combat damage to a player, you may pay {X}. When you do, ..." X is the payer's to choose as
/// they pay (CR 107.3f), 0 included; paying it, whatever X, is doing it, and the reflexive ability (CR 603.12) triggers carrying
/// that X and the player the damage was dealt to. Declining is not doing it: nothing triggers.
fn combat_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if let Some(x) = g.may_pay_x(you, item.object, item.object, cost("{X}"), &PAY_X)? {
        let me = g.card_id(&BACK);
        g.reflexive_trigger_counted(item.object, you, AbilityRef::delayed(me, 0), item.subject, x);
    }
    Ok(())
}

/// "... put any number of target creature cards with total mana value X or less from that player's graveyard onto the
/// battlefield under your control." The total was judged as the targets were chosen (CR 601.2c, 603.3d) and is judged again
/// on resolution: if it has grown past X every target is illegal (CR 608.2b); otherwise the targets still legal enter under
/// the ability's controller's control (CR 110.2a).
fn reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let cards: Vec<(ObjectId, PlayerId)> = targets::legal_objects(g, item).into_iter().map(|c| (c, you)).collect();
    g.put_all_onto_battlefield(&cards)?;
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        firebending::trigger::<3>("Firebending 3 (Whenever this creature attacks, add {R}{R}{R}. This mana lasts until end of combat.)"),
        triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT, combat_damage, "Whenever Fire Lord Sozin deals combat damage to a player, you may pay {X}. When you do, put any number of target creature cards with total mana value X or less from that player's graveyard onto the battlefield under your control."),
    ],
    delayed: &[delayed_trigger(reanimate, "When you do, put any number of target creature cards with total mana value X or less from that player's graveyard onto the battlefield under your control.")
        .targets(&[TargetSpec { min: 0, max: TargetSpec::ANY_NUMBER, ..target(targets::creature_card_in_that_players_graveyard) }
            .together(select::total_mana_value_at_most(TargetSpec::COUNTED))
            .labelled("Creature cards from that player's graveyard to put onto the battlefield under your control")])],
    ..oracle::FIRE_LORD_SOZIN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::destroy_all_creatures, "I \u{2014} Destroy all creatures."),
        chapter(2, exile_up_to_four_named_from_graveyard_hand_and_library, "II \u{2014} Choose a card name. Search target opponent's graveyard, hand, and library for up to four cards with that name and exile them. Then that player shuffles.")
            .targets(&[targets::OPPONENT]),
        chapter(3, chapter_three, "III \u{2014} Exile this Saga, then return it to the battlefield transformed under your control."),
    ],
    back: Some(&BACK),
    ..oracle::THE_RISE_OF_SOZIN
};
