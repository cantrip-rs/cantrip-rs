use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target Equipment, instant, or sorcery card from your graveyard."
fn equipment_instant_or_sorcery_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[]) && matches!(t, Target::Object(id) if {
        let c = q.chars(id);
        c.is(CardType::Instant) || c.is(CardType::Sorcery) || c.has_subtype(Subtype::Equipment)
    })
}

/// Exile the target card; until the end of its controller's next turn they may play it (CR 601.3, 305.1). The
/// permission is for the card that actually reaches exile (CR 400.7): a replacement that sent it elsewhere leaves
/// nothing to play.
fn exile_and_play(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let exiled = g.exile(card);
    if g.obj(exiled).zone.is(ZoneKind::Exile) {
        g.allow_play(
            exiled,
            item.controller,
            PlayGrant { instead: None, lands: true, expiry: Expiry::EndOfYourNextTurn, ..PlayGrant::CAST },
        );
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, exile_and_play, "When Thor enters, exile target Equipment, instant, or sorcery card from your graveyard. Until the end of your next turn, you may play that card.")
            .targets(&[target(equipment_instant_or_sorcery_in_your_graveyard)]),
        triggered(triggers::NONCREATURE_SPELL_YOU_CAST_ABOUT_IT, effects::damage_first_target_equal_to_the_spells_mana_value, "Whenever you cast a noncreature spell, Thor deals damage equal to that spell's mana value to any target.")
            .targets(&[targets::ANY]),
    ],
    ..oracle::THOR_GOD_OF_THUNDER
};
