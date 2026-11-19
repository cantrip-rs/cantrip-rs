use cantrip_api::*;
use cantrip_mechanics::{choose, graveyards, select, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "When Valki enters, each opponent reveals their hand. For each opponent, exile a creature card they revealed this way until
/// Valki leaves the battlefield." (CR 701.20, 608.2c, 610.3, 607.2a): every opponent reveals first, then the controller chooses
/// a creature card from each revealed hand, and the cards chosen are exiled together, linked to Valki. A hand with no creature
/// card gives nothing to choose; nothing is exiled if Valki has already left (CR 610.3b).
fn reveal_and_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let hands: Vec<Vec<ObjectId>> = opponents.iter().map(|&p| g.state().zone(Zone::hand(p)).to_vec()).collect();
    for (&opponent, hand) in opponents.iter().zip(&hands) {
        if !hand.is_empty() {
            g.reveal(opponent, hand);
        }
    }
    let mut chosen = Vec::new();
    for hand in hands {
        chosen.extend(choose::select(g, item.controller, item.object, Purpose::Exile, hand, select::creature_cards(select::as_many_as_can(1)))?);
    }
    g.exile_with(&chosen, g.linked(item), true);
    Ok(())
}

/// "{X}: Choose a creature card exiled with Valki with mana value X. Valki becomes a copy of that card." (CR 707.2, 712.8a):
/// chosen as the ability resolves, not targeted (the rulings), among the cards exiled with this Valki; nothing happens when none
/// has mana value X. The card's copiable values, as printed, with no exception, so Valki keeps none of its own text; it stays the
/// same object (CR 707.4), so the cards stay exiled until it leaves the battlefield (CR 610.3).
fn become_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    let rule = select::creature_cards(select::as_many_as_can(1)).with(select::mana_value_exactly(item.x as u32));
    // With no such card there is nothing to choose, and nothing happens (the rulings).
    if !cantrip_api::select::first_answer(&*g, &rule, item.controller, item.object, &exiled, true).is_some_and(|cards| !cards.is_empty()) {
        return Ok(());
    }
    let Some(&card) = choose::select(g, item.controller, item.object, Purpose::CopyWhich, exiled, rule)?.first() else { return Ok(()) };
    let values = g.copiable_values(card);
    g.become_copy(item.object, item.object, values, Duration::Indefinite);
    Ok(())
}

/// "You may play cards exiled with Tibalt, Cosmic Impostor, and you may spend mana as though it were mana of any color to cast those
/// spells." (CR 114.2, 609.4b; linked to each of Tibalt's exiling abilities by CR 607.1d, 607.2a, 607.4): the Tibalt that made this emblem (`Query::made_by`), whether or not he is still there (the rulings),
/// for its controller; a land card is played as a land play.
fn exiled_with_its_tibalt(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    player == asker.controller && q.made_by(asker.source).is_some_and(|by| q.exiled_with(by).contains(&card))
}

pub static EMBLEM: CardDef = CardDef {
    name: "Tibalt, Cosmic Impostor emblem",
    has_cost: false,
    text: "You may play cards exiled with Tibalt, Cosmic Impostor, and you may spend mana as though it were mana of any color to cast those spells.",
    exile_casts: &[ExileCast {
        text: "Play a card exiled with Tibalt",
        from: ZoneKind::Exile,
        allows: exiled_with_its_tibalt,
        additional: Cost::NONE,
        any_type: false,
        any_color: true,
        lands: true,
        life_for_mana_value: false,
        instead: None,
        spell_replacement: None,
    }],
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];

/// "As Tibalt enters, you get an emblem with ..." (CR 614.1c, 114.2): made as he arrives, naming this new object.
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    tokens::emblem_as_enters(g, entering, &EMBLEM);
    Ok(())
}

/// "+2: Exile the top card of each player's library." (CR 608.2c, 607.2a): all together, linked to this Tibalt.
fn exile_each_top_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tops: Vec<ObjectId> = g.state().apnap().filter_map(|p| g.state().zone(Zone::library(p)).first().copied()).collect();
    g.exile_with(&tops, g.linked(item), false);
    Ok(())
}

/// "-3: Exile target artifact or creature."
fn exile_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile_with(&[target], g.linked(item), false);
    Ok(())
}

/// "-8: Exile all graveyards. Add {R}{R}{R}." (CR 406.2, 106.4): the cards are exiled with Tibalt (CR 607.2a), together, and the
/// mana is added even if there were none (the rulings).
fn exile_graveyards_add_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<ObjectId> = graveyards::cards_in_graveyards(g.state()).collect();
    g.exile_with(&cards, g.linked(item), false);
    g.add_mana(item.controller, Mana::R, 3);
    Ok(())
}

pub static BACK: CardDef = CardDef {
    enters: Some(enters),
    abilities: &[
        ability(loyalty(2), exile_each_top_card, "+2: Exile the top card of each player's library."),
        ability(loyalty(-3), exile_target, "\u{2212}3: Exile target artifact or creature.").targets(&[targets::ARTIFACT_OR_CREATURE]),
        ability(loyalty(-8), exile_graveyards_add_red, "\u{2212}8: Exile all graveyards. Add {R}{R}{R}.").produces_mana(),
    ],
    ..oracle::TIBALT_COSMIC_IMPOSTOR
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, reveal_and_exile, "When Valki enters, each opponent reveals their hand. For each opponent, exile a creature card they revealed this way until Valki leaves the battlefield."),
        ability(cost("{X}"), become_copy, "{X}: Choose a creature card exiled with Valki with mana value X. Valki becomes a copy of that card."),
    ],
    back: Some(&BACK),
    ..oracle::VALKI_GOD_OF_LIES
};
