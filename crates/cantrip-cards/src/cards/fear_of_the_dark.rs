use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// An intervening "if defending player controls no Glimmer creatures" (CR 603.4), checked as the ability triggers and again as it
/// resolves: only a Glimmer that is a creature counts (Enduring Tenacity as an enchantment does not). The defending player is
/// the one this creature attacked, still known once it is no longer attacking (CR 508.5).
fn defending_player_controls_no_glimmer_creatures(q: &dyn Query, asker: &Asker) -> bool {
    let glimmer = q.words(asker.source).subtype(Subtype::Glimmer);
    !q.state().battlefield().iter().any(|&id| {
        q.defending_player(asker.source) == Some(q.controller(id)) && q.chars(id).is(CardType::Creature) && q.chars(id).has_subtype(glimmer)
    })
}

/// "It gains menace and deathtouch until end of turn" (CR 611.2a): one layer 6 effect on this creature for the rest of the turn;
/// a creature that is no longer the object it was gains nothing (CR 400.7).
fn gains_menace_and_deathtouch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::AddKeywords(Keywords::of(&[Keyword::Menace, Keyword::Deathtouch])), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        gains_menace_and_deathtouch,
        "Whenever this creature attacks, if defending player controls no Glimmer creatures, it gains menace and deathtouch until end of turn. (A creature with menace can't be blocked except by two or more creatures.)",
    )
    .condition(defending_player_controls_no_glimmer_creatures)],
    ..oracle::FEAR_OF_THE_DARK
};
