use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Choose one — • Counter target spell with mana value less than or equal to the number of cards in its controller's graveyard. • Destroy target creature with mana value less than or equal to the number of cards in its controller's graveyard." (CR 115.1).
fn choose_one_counter_target_spell_mode_0_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if (q.state().obj(id).zone.kind() == ZoneKind::Stack) &&
        ((i64::from(q.mana_value(id))) <= (q.state().apnap().collect:: < Vec < PlayerId >
        > ().into_iter().flat_map(| p : PlayerId | q.state().zone(Zone::graveyard(p))
        .to_vec()).collect:: < Vec < ObjectId > > ().into_iter().filter(| & card_in_zone
        | q.state().obj(card_in_zone).became.is_none() && q.state().obj(card_in_zone)
        .zone.kind() == ZoneKind::Graveyard && Some(id).map(| o : ObjectId | q
        .controller(o)) == Some(q.state().obj(card_in_zone).owner)).collect:: < Vec <
        ObjectId > > ().len() as i64))
    )
}

/// "Choose one — • Counter target spell with mana value less than or equal to the number of cards in its controller's graveyard. • Destroy target creature with mana value less than or equal to the number of cards in its controller's graveyard." (CR 608.2, 701.5a).
fn choose_one_counter_target_spell_mode_0(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(
        g,
        |g| {
            g.counter(target);
        },
    );
    Ok(())
}

/// "Choose one — • Counter target spell with mana value less than or equal to the number of cards in its controller's graveyard. • Destroy target creature with mana value less than or equal to the number of cards in its controller's graveyard." (CR 115.1).
fn choose_one_counter_target_spell_mode_1_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) && (i64::from(q.mana_value(id))) <= (q.state()
        .apnap().collect:: < Vec < PlayerId > > ().into_iter().flat_map(| p : PlayerId |
        q.state().zone(Zone::graveyard(p)).to_vec()).collect:: < Vec < ObjectId > > ()
        .into_iter().filter(| & card_in_zone | q.state().obj(card_in_zone).became
        .is_none() && q.state().obj(card_in_zone).zone.kind() == ZoneKind::Graveyard &&
        Some(id).map(| o : ObjectId | q.controller(o)) == Some(q.state()
        .obj(card_in_zone).owner)).collect:: < Vec < ObjectId > > ().len() as i64)))
    )
}

static ABILITY_0_MODES: ModalDef = modes(
        &[
            mode(
                    choose_one_counter_target_spell_mode_0,
                    "Counter target spell with mana value less than or equal to the number of cards in its controller's graveyard.",
                )
                .targets(&[target(choose_one_counter_target_spell_mode_0_target_0)]),
            mode(
                    effects::destroy_first_target,
                    "Destroy target creature with mana value less than or equal to the number of cards in its controller's graveyard.",
                )
                .targets(&[target(choose_one_counter_target_spell_mode_1_target_0)]),
        ],
    )
    .choose(1, 1);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&ABILITY_0_MODES),
    ..oracle::DROWN_IN_THE_LOCH
};
