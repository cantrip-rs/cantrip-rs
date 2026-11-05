use cantrip_api::*;
use cantrip_mechanics::triggers::{BEGINNING_OF_YOUR_COMBAT, THIS_ENTERS};
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "GiveControl",
    "have the targeted opponent gain control of the targeted permanent?",
    "They keep it; you create an Ally token with a +1/+1 counter for each permanent you own that your opponents control",
    "Give it away",
    "Don't",
);

/// "When Iroh enters, create a Food token." (CR 111.10b.)
fn create_food(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_food(g, item.controller)
}

/// "You may have target opponent gain control of target permanent you control." (CR 611.2, 608.2b.) Gaining control has no
/// duration, so it lasts as long as the permanent stays on the battlefield. If either target is illegal as this resolves the
/// ability does nothing and the choice is not offered (the card's ruling). "When you do, ..." (CR 603.12) is a reflexive
/// trigger, put on the stack only if the opponent was given the permanent.
fn give_control(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let (Some(opponent), Some(permanent)) = (targets::player(g, item, 0), targets::object(g, item, 1)) else { return Ok(()) };
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    // "When you do": only if the opponent now controls it - not if they have left the game (CR 800.4b).
    if g.gain_control(item.object, permanent, opponent, Duration::Indefinite) {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "Create a 1/1 white Ally creature token. Put a +1/+1 counter on that token for each permanent you own that your
/// opponents control." (CR 111.1, 122.1.) Counted as the reflexive ability resolves, by owner and current controller
/// (CR 108.3, 110.2); a token is owned by the player who created it.
fn ally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let opponents: Vec<PlayerId> = g.state().opponents(you).collect();
    let owned = opponents.into_iter().flat_map(|p| g.permanents_of(p)).filter(|&id| g.obj(id).owner == you).count();
    for token in tokens::create(g, you, &tokens::ALLY, 1)? {
        g.add_counters(token, CounterKind::PlusOne, i64::from(owned as i16));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_food, "When Iroh enters, create a Food token."),
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            give_control,
            "At the beginning of combat on your turn, you may have target opponent gain control of target permanent you control. When you do, create a 1/1 white Ally creature token. Put a +1/+1 counter on that token for each permanent you own that your opponents control.",
        )
        .targets(&[targets::OPPONENT, targets::PERMANENT_YOU_CONTROL]),
    ],
    delayed: &[delayed_trigger(
        ally,
        "When you do, create a 1/1 white Ally creature token. Put a +1/+1 counter on that token for each permanent you own that your opponents control.",
    )],
    ..oracle::IROH_TEA_MASTER
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
