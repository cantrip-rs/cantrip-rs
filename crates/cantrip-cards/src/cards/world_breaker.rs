use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}{C}, Sacrifice a land: Return this card from your graveyard to your hand. ({C} represents colorless mana.)" (CR 604.1).
fn return_card_graveyard_hand_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.colors = ColorSet::NONE;
}

/// "When you cast this spell, exile target artifact, enchantment, or land." (CR 115.1).
fn exile_target_artifact_enchantment_land_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Enchantment) || q
        .chars(id).is(CardType::Land)))
    )
}

/// "When you cast this spell" (CR 603.2).
fn when_cast_spell(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if asker.controller == *player && asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "When you cast this spell, exile target artifact, enchantment, or land." (CR 608.2, 406.3).
fn exile_target_artifact_enchantment_land(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile_all(&[target]);
    Ok(())
}

/// "{2}{C}, Sacrifice a land: Return this card from your graveyard to your hand." (CR 608.2).
fn return_card_graveyard_hand(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Land)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::SPELL_CAST, when_cast_spell),
                exile_target_artifact_enchantment_land,
                "When you cast this spell, exile target artifact, enchantment, or land.",
            )
            .targets(&[target(exile_target_artifact_enchantment_land_target_0)])
            .functions_in(ZoneKinds::of(&[ZoneKind::Stack])),
        ability(
                cost("{2}{C}").and(sacrifice(1, return_card_graveyard_hand)),
                effects::return_this_to_hand,
                "{2}{C}, Sacrifice a land: Return this card from your graveyard to your hand.",
            )
            .from(ZoneKind::Graveyard),
    ],
    statics: &[
        characteristic_defining(
            Layer::Color,
            return_card_graveyard_hand_apply_0,
            "Devoid",
        ),
    ],
    ..oracle::WORLD_BREAKER
};
