use cantrip_api::*;
use cantrip_mechanics::statics::{itself_with_threshold, plus_pt};
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "An instant or sorcery card" (CR 205.2a): what the card put into the hand must be.
fn is_instant_or_sorcery(q: &dyn Query, card: ObjectId) -> bool {
    q.chars(card).is(CardType::Instant) || q.chars(card).is(CardType::Sorcery)
}

fn instant_or_sorcery(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| is_instant_or_sorcery(q, card))
}

/// "Mill four cards, then put an instant or sorcery card from among them into your hand." (CR 701.17a, 701.17c): the cards
/// milled are the ones it takes from, as they are where they went (a graveyard, or exile by a replacement effect); the card is
/// not optional, so one is chosen whenever any milled card is an instant or sorcery, and none is asked when none is.
fn speak_secrets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let among = g.mill(you, 4);
    if !among.iter().any(|&card| is_instant_or_sorcery(g, card)) {
        return Ok(());
    }
    let rule = Selection::new(select::as_many_as_can(1)).with(select::part(&(instant_or_sorcery as SelectFn), 0, "")).called("instant or sorcery");
    let chosen = choose::select(g, you, item.object, Purpose::PutIntoHand, among, rule)?;
    g.return_to_hand(&chosen);
    Ok(())
}

/// The Adventure (CR 715): Speak Secrets.
pub static SPEAK_SECRETS: CardDef = CardDef { spell: spell(&[], speak_secrets), ..oracle::SPEAK_SECRETS };

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself_with_threshold,
        plus_pt::<1, 1>,
        "Threshold \u{2014} This creature gets +1/+1 as long as there are seven or more cards in your graveyard.",
    )],
    adventure: Some(&SPEAK_SECRETS),
    ..oracle::MOST_DECREPIT_OLD_BIRD
};

pub static EXTRA: &[&CardDef] = &[&SPEAK_SECRETS];
