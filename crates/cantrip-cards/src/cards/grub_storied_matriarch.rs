use cantrip_api::*;
use cantrip_mechanics::{blight, combat, effects, targets, triggers};
use cantrip_oracle as oracle;

const BLIGHT_ONE: Question = Question::new("BlightOne", "blight 1?", "", "Blight 1", "Don't");

/// "Target Goblin card in your graveyard" (CR 115.1, 404.1): the Goblin a card in a graveyard is, as the words of the effect
/// that returns it say (CR 612: Mistform Ultimus).
fn goblin_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Goblin)))
}

/// "Return up to one target Goblin card from your graveyard to your hand." With no target chosen it does nothing; with one that
/// is gone as it would resolve it does not resolve at all (CR 608.2b), which the core decides.
fn return_a_goblin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.return_to_hand(&[card]);
    Ok(())
}

/// "At the beginning of your first main phase, you may pay {R}. If you do, transform Grub."
fn pay_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{R}"))
}

/// "At the beginning of your first main phase, you may pay {B}. If you do, transform Grub."
fn pay_black(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{B}"))
}

/// The ability each token copy has by the copy's exception (CR 707.9a). Not a card anyone plays: a definition for the
/// exception's ability to be referred to by. It is part of the copy's copiable values (CR 707.9a), so a copy of the token has
/// it too.
pub static EXCEPTION: CardDef = CardDef {
    name: "Grub, Notorious Auntie (copy exception)",
    abilities: &[triggered(triggers::EACH_END_STEP, effects::sacrifice_self, "At the beginning of the end step, sacrifice this token.")],
    ..CardDef::BLANK
};

/// "You may blight 1. If you do, create a tapped and attacking token that's a copy of the blighted creature, except it has 'At
/// the beginning of the end step, sacrifice this token.'" (CR 701.68a, 707.2, 707.9a, 111.1, 508.4): the copy is of the
/// creature that was blighted, as it is now, counters not copied (CR 707.2), and is created tapped (CR 614.1c). It was not
/// declared as an attacker, so nothing that triggers on attackers sees it, and its controller chooses what it attacks (CR 508.4).
fn blight_and_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(blighted) = blight::may_in_effect(g, item.controller, item.object, 1, &BLIGHT_ONE)? else { return Ok(()) };
    let exception = AbilityRef { card: g.card_id(&EXCEPTION), index: 0 };
    let values = g.copiable_values(blighted).with_ability(exception);
    let defender = combat::choose_attack_target(g, item.controller, item.object)?;
    let tokens = g.create_token_copies_tapped(values, item.controller, 1, true)?;
    if let Some(defender) = defender {
        g.enter_attacking(&tokens, defender);
    }
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, blight_and_copy, "Whenever Grub attacks, you may blight 1. If you do, create a tapped and attacking token that's a copy of the blighted creature, except it has \"At the beginning of the end step, sacrifice this token.\""),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_black, "At the beginning of your first main phase, you may pay {B}. If you do, transform Grub."),
    ],
    ..oracle::GRUB_NOTORIOUS_AUNTIE
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS_OR_TRANSFORMS, return_a_goblin, "Whenever this creature enters or transforms into Grub, Storied Matriarch, return up to one target Goblin card from your graveyard to your hand.")
            .targets(&[target(goblin_card_in_your_graveyard).optional()]),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_red, "At the beginning of your first main phase, you may pay {R}. If you do, transform Grub."),
    ],
    back: Some(&BACK),
    ..oracle::GRUB_STORIED_MATRIARCH
};

pub static EXTRA: &[&CardDef] = &[&EXCEPTION];
