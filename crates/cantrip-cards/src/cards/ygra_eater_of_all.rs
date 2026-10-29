use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::grant_delayed_ability;
use cantrip_mechanics::triggers::ward_sacrifice_a_food;
use cantrip_oracle as oracle;

const OTHER_CREATURES: &str =
    "Other creatures are Food artifacts in addition to their other types and have \"{2}, {T}, Sacrifice this permanent: You gain 3 life.\"";

/// "Other creatures": every creature but Ygra, whoever controls it, as the effects applied before this one leave it. One that
/// is a creature only because of another effect of layer 4 - a crewed Vehicle, an animated artifact or land, Kaito on his
/// turn - is one whatever their timestamps: this effect depends on that one (CR 613.8a), which applies first.
fn other_creatures(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    affected != source && so_far.is(CardType::Creature)
}

/// Layer 4 (CR 613.1d, 205.1b): "are Food artifacts in addition to their other types" - the card type artifact and the artifact
/// type Food, added, and nothing taken away.
fn food_artifacts_too(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.types = chars.types.with(CardType::Artifact);
    chars.subtypes = chars.subtypes.with(Subtype::Food);
}

/// "Whenever a Food is put into a graveyard from the battlefield" (CR 603.6c, 603.10a): any Food, anyone's, token or card,
/// looked back on as it last existed on the battlefield - a creature that was a Food only because of Ygra counts when it
/// dies with Ygra. Each Food is its own event, so several at once trigger as many times (CR 603.2c).
fn food_put_into_graveyard(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard) && q.chars(*old).has_subtype(Subtype::Food) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward_sacrifice_a_food("Ward\u{2014}Sacrifice a Food."),
        triggered(
            when(EventMask::ZONE_CHANGE, food_put_into_graveyard),
            effects::two_plus_one_counters_on_this,
            "Whenever a Food is put into a graveyard from the battlefield, put two +1/+1 counters on Ygra.",
        ),
    ],
    delayed: &[ability(cost("{2}, {T}").and(SACRIFICE_THIS), effects::gain_three_life, "{2}, {T}, Sacrifice this permanent: You gain 3 life.")],
    statics: &[
        static_ability(Layer::Type, other_creatures, food_artifacts_too, OTHER_CREATURES),
        // CR 613.6: the effect that began in layer 4 applies to the same creatures in layer 6.
        static_ability(Layer::Ability, other_creatures, grant_delayed_ability, OTHER_CREATURES).continues(),
    ],
    ..oracle::YGRA_EATER_OF_ALL
};
