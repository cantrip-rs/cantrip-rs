use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{choose, effects, history, select, targets, tokens, top_of_library};
use cantrip_oracle as oracle;

/// "If you would pay life while your library has at least that many cards in it" (CR 119.4, 614.1a): a payment of life by
/// Ashiok's controller - a cost's or a resolving instruction's, not any other loss - of an amount their library holds as many
/// cards as. With fewer cards the life is paid as normal (the 2023-09-01 ruling). An amount above their life total is no
/// payment to replace: it can't be paid at all (`actions::can_pay_life`), so no one asks.
fn you_pay_life_with_that_many_cards_in_your_library(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::LifeChange(l) = proposal else { return false };
    l.paid && l.player == asker.controller && l.amount < 0 && q.state().zone(Zone::library(l.player)).len() as i64 >= -i64::from(l.amount)
}

/// "... instead": nothing of the life is paid; the cards are exiled in its place (`exile_that_many_from_the_top`). It is not
/// optional and the payment is not split (the 2023-09-01 ruling).
fn pay_no_life(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::LifeChange(l) = proposal {
        l.amount = 0;
    }
}

/// "... exile that many cards from the top of your library": as many as the life would have been, as the effects applied
/// before it have left it (CR 616.1f), together (CR 101.4).
fn exile_that_many_from_the_top(g: &mut dyn Game, _: &Asker, before: &Proposal, _: &Proposal) {
    let Proposal::LifeChange(l) = before else { return };
    let top = top_of_library(g, l.player, l.amount.unsigned_abs() as usize);
    g.exile_all(&top);
}

/// "+1: Look at the top two cards of your library. Exile one of them and put the other into your hand" (CR 701.20e, 701.13):
/// you see both and choose the one to exile; the other goes to your hand once it is gone. A library of one gives the card to
/// exile and nothing for the hand (CR 609.3).
fn exile_one_of_the_top_two_the_other_to_your_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let looked = top_of_library(g, you, 2);
    g.look_at(you, &looked);
    let chosen = choose::select(g, you, item.object, Purpose::Exile, looked.clone(), select::cards(select::as_many_as_can(1)))?;
    g.exile_all(&chosen);
    let rest: Vec<ObjectId> = looked.into_iter().filter(|c| !chosen.contains(c)).collect();
    g.return_to_hand(&rest);
    Ok(())
}

/// "−2: Create two 1/1 black Nightmare creature tokens ..." (CR 701.7a): both at once.
fn create_two_nightmares(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &NIGHTMARE, 2)?;
    Ok(())
}

/// "−7: Target player exiles the top X cards of their library, where X is the total mana value of cards you own in exile"
/// (CR 107.3, 202.3): X counted as the ability resolves, over every card in exile that Ashiok's controller owns, whoever
/// exiled it and whichever way up; the player exiles them themself, together. A target no longer legal does nothing.
fn target_player_exiles_x_from_the_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let x: u32 = g.state().seats().flat_map(|p| g.state().zone(Zone::exile(p)).to_vec()).filter(|&c| g.obj(c).owner == item.controller).map(|c| g.mana_value(c)).sum();
    let top = top_of_library(g, player, x as usize);
    g.exile_all(&top);
    Ok(())
}

/// The 1/1 black Nightmare creature token Ashiok makes: "At the beginning of combat on your turn, if a card was put into exile
/// this turn, put a +1/+1 counter on this token."
static NIGHTMARE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        effects::plus_one_counter_on_this,
        "At the beginning of combat on your turn, if a card was put into exile this turn, put a +1/+1 counter on this token.",
    )
    .condition(history::a_card_was_put_into_exile_this_turn)],
    ..token!(Nightmare)
};

pub static CARD: CardDef = CardDef {
    replacements: &[replacement(
        ReplacementKind::LifeChange,
        you_pay_life_with_that_many_cards_in_your_library,
        pay_no_life,
        "exiles that many cards from the top of your library instead of paying life",
    )
    .then(exile_that_many_from_the_top)],
    abilities: &[
        ability(loyalty(1), exile_one_of_the_top_two_the_other_to_your_hand, "+1: Look at the top two cards of your library. Exile one of them and put the other into your hand."),
        ability(
            loyalty(-2),
            create_two_nightmares,
            "\u{2212}2: Create two 1/1 black Nightmare creature tokens with \"At the beginning of combat on your turn, if a card was put into exile this turn, put a +1/+1 counter on this token.\"",
        ),
        ability(
            loyalty(-7),
            target_player_exiles_x_from_the_top,
            "\u{2212}7: Target player exiles the top X cards of their library, where X is the total mana value of cards you own in exile.",
        )
        .targets(&[targets::PLAYER]),
    ],
    ..oracle::ASHIOK_WICKED_MANIPULATOR
};

pub static EXTRA: &[&CardDef] = &[&NIGHTMARE];
