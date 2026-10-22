use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, bestow, tokens, triggers::LANDFALL};
use cantrip_oracle as oracle;

const COPY: Question = Question::new(
    "CreateCopyToken",
    "pay {cost} to create a token that's a copy of {subject}?",
    "If you don't, you create a 1/1 green Insect",
    "Pay {cost}",
    "Make an Insect",
);

/// The 1/1 green Insect creature token it makes.
pub static INSECT: CardDef = CardDef {
    name: "Insect Token",
    token_name: Some(&[Subtype::Insect]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Insect]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};

const STATICS: &[StaticDef] = &[static_ability(Layer::PtModify, attached_object, cantrip_mechanics::statics::plus_pt::<1, 1>, "Enchanted creature gets +1/+1.")];

/// "You may pay {1}{G} if this permanent is attached to a creature you control. If you do, create a token that's a copy
/// of that creature. If you didn't create a token this way, create a 1/1 green Insect creature token." Read the CR 608.2h
/// way: this permanent and the creature as they are, or as they last were if they have left. The payment is asked only
/// while attached to a creature its controller controls.
fn landfall_effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    let host = g.obj(me).attached_to;
    let mut created = false;
    if let Some(host) = host
        && g.chars(host).is(CardType::Creature)
        && g.obj(host).controller == item.controller
        && g.may_pay(item.controller, me, host, cost("{1}{G}"), &COPY)?
    {
        let values = g.copiable_values(host);
        created = !g.create_token_copies_of(values, item.controller, 1)?.is_empty();
    }
    if !created {
        tokens::create(g, item.controller, &INSECT, 1)?;
    }
    Ok(())
}

const ABILITIES: &[AbilityDef] = &[triggered(LANDFALL, landfall_effect, "Landfall \u{2014} Whenever a land you control enters, you may pay {1}{G} if this permanent is attached to a creature you control. If you do, create a token that's a copy of that creature. If you didn't create a token this way, create a 1/1 green Insect creature token.")];

/// Bestow {1}{G} (CR 702.103): while it is bestowed it is an Aura enchantment with "enchant creature", not a creature,
/// and has the same abilities.
pub static BESTOWED: CardDef = bestow::bestowed(CardDef {
        abilities: ABILITIES,
        statics: STATICS,
        ..oracle::SPRINGHEART_NANTUKO
});

pub static CARD: CardDef = CardDef {
    abilities: ABILITIES,
    statics: STATICS,
    bestow: Some(&BESTOWED),
    bestow_cost: mana("{1}{G}"),
    ..oracle::SPRINGHEART_NANTUKO
};

pub static EXTRA: &[&CardDef] = &[&INSECT, &BESTOWED];
