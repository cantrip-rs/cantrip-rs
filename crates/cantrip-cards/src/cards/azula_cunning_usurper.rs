use cantrip_api::*;
use cantrip_mechanics::{choose, firebending, select, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target opponent exiles a nontoken creature they control, then they exile a nonland card from their graveyard": the
/// opponent chooses each (CR 608.2d), a creature if they control a nontoken one and a card if their graveyard holds a nonland
/// one, and both are exiled with Azula (CR 607.2a), which the permission below reads. A card is chosen from the graveyard
/// after the creature is exiled, so a creature that died is not among them but a card the exile itself put there would be.
fn exile_a_creature_then_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else {
        return Ok(());
    };
    let by = g.linked(item);
    let creatures = {
        let q: &dyn Query = g;
        q.state().battlefield().iter().copied().filter(|&id| q.controller(id) == opponent && !q.state().obj(id).token && q.chars(id).is(CardType::Creature)).collect::<Vec<_>>()
    };
    let chosen = choose::select(g, opponent, item.object, Purpose::Exile, creatures, Selection::of(select::as_many_as_can(1), &[]))?;
    g.exile_with(&chosen, by, false);
    let cards = {
        let q: &dyn Query = g;
        q.state().zone(Zone::graveyard(opponent)).iter().copied().filter(|&id| !q.chars(id).is(CardType::Land)).collect::<Vec<_>>()
    };
    let chosen = choose::select(g, opponent, item.object, Purpose::Exile, cards, Selection::of(select::as_many_as_can(1), &[]))?;
    g.exile_with(&chosen, by, false);
    Ok(())
}

/// "You may cast cards exiled with Azula" (CR 607.2a, 601.3): a card exiled with this Azula, by whoever's choice, and not one
/// exiled with an earlier Azula (CR 400.7). It is Azula's controller who may cast it, on their own turn only.
fn exiled_with_azula(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    player == asker.controller && q.exiled_with(q.linked_to(asker)).contains(&card)
}

/// "During your turn, you may cast cards exiled with Azula" (CR 601.3).
fn during_your_turn(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    q.state().turn.active == player && exiled_with_azula(q, asker, card, player)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        firebending::trigger::<2>("Firebending 2 (Whenever this creature attacks, add {R}{R}. This mana lasts until end of combat.)"),
        triggered(THIS_ENTERS, exile_a_creature_then_a_card, "When Azula enters, target opponent exiles a nontoken creature they control, then they exile a nonland card from their graveyard.")
            .targets(&[targets::OPPONENT]),
    ],
    exile_casts: &[ExileCast {
        text: "Cast from exile",
        from: ZoneKind::Exile,
        allows: during_your_turn,
        additional: Cost::NONE,
        any_type: true,
        any_color: false,
        lands: false,
        life_for_mana_value: false,
        instead: None,
        spell_replacement: None,
    }],
    flash_rules: &[during_your_turn],
    ..oracle::AZULA_CUNNING_USURPER
};
