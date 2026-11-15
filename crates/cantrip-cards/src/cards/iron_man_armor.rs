use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::{have_flying, plus_one_plus_one_per_artifact_you_control};
use cantrip_mechanics::{Animation, animate, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

fn plus_two_plus_one(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = chars.power.map(|v| v + 2);
    chars.toughness = chars.toughness.map(|v| v + 1);
}

/// "{2}: If this Equipment isn't a creature, it becomes a 0/0 Construct Hero artifact creature with flying and 'This creature
/// gets +1/+1 for each artifact you control' until end of turn." The check is made as the ability resolves (CR 608.2c), so a second activation
/// in one turn does nothing. It is an artifact already, and an Equipment, and keeps both: Artifact and Creature are added to
/// its types, and Construct and Hero (read through its words, CR 612) replace any creature types it had, since the text
/// does not say it is still what it was (CR 205.1b; layer 4); its base power and toughness are set (7b) and the flying and the quoted ability granted (6), so the Armor counts itself among the artifacts. An
/// Equipment that has left the battlefield is a new object the ability does not affect (CR 400.7).
fn becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    if g.chars(me).is(CardType::Creature) {
        return Ok(());
    }
    let words = g.words(me);
    let animation = Animation {
        power: 0,
        toughness: 0,
        colors: None,
        types: TypeSet::of(&[CardType::Artifact]),
        subtypes: &[words.subtype(Subtype::Construct), words.subtype(Subtype::Hero)],
        all_creature_types: false,
        replaces_creature_types: true,
        keywords: Keywords::of(&[Keyword::Flying]),
        ability: Some(AbilityRef::delayed(item.ability.card, 0)),
    };
    animate(g, me, me, &animation, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, attach it to target creature you control.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        ability(cost("{2}"), becomes_creature, "{2}: If this Equipment isn't a creature, it becomes a 0/0 Construct Hero artifact creature with flying and \"This creature gets +1/+1 for each artifact you control\" until end of turn."),
        equip(mana("{2}"), "Equip {2}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_two_plus_one, "Equipped creature gets +2/+1."),
        static_ability(Layer::Ability, attached_object, have_flying, "Equipped creature has flying."),
    ],
    delayed: &[granted_static(Layer::PtModify, plus_one_plus_one_per_artifact_you_control, "This creature gets +1/+1 for each artifact you control.")],
    ..oracle::IRON_MAN_ARMOR
};
