use cantrip_api::*;
use cantrip_mechanics::{effects, mana, triggers};
use cantrip_oracle as oracle;

const TAP_OR_SACRIFICE: Question = Question::new("TapPermanent", "tap an untapped artifact or land you control, or sacrifice {subject}?", "", "Tap it", "Sacrifice it");

/// "An untapped artifact or land you control" as the permanent the payment taps (CR 118.3): one that came under your control this
/// turn counts, as a creature's {T} symbol is the only thing summoning sickness limits (CR 302.6). The Thoroughfare itself is
/// one only if something has untapped it since it entered tapped.
fn untapped_artifact_or_land_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && !q.obj(id).tapped && (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Land))
}

/// "When this land enters, sacrifice it unless you tap an untapped artifact or land you control" (CR 118.12a): the payment is made
/// as the ability resolves, and when it is not made the land is sacrificed.
fn sacrifice_unless_you_tap_an_artifact_or_land(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, Cost::tap_untapped(1, untapped_artifact_or_land_you_control), &TAP_OR_SACRIFICE)? {
        return effects::sacrifice_self(g, item);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            sacrifice_unless_you_tap_an_artifact_or_land,
            "When this land enters, sacrifice it unless you tap an untapped artifact or land you control.",
        ),
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
    ],
    ..oracle::PUBLIC_THOROUGHFARE
};
