use cantrip_api::*;
use cantrip_mechanics::impending;
use cantrip_oracle as oracle;

/// "Whenever this permanent enters or attacks" (CR 603.2).
fn when_permanent_enters_attacks(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    if let Event::ZoneChange { new, to, .. } = event {
        let id = *new;
        let subject = Target::Object(id);
        if to.is(ZoneKind::Battlefield) && asker.source == id {
            return Fired::about(subject);
        }
    }
    let Event::Attacks { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    if asker.source == id {
        return Fired::about(subject);
    }
    None
}

pub static EVERYWHERE: CardDef = CardDef {
    name: "Everywhere",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    subtypes: SubtypeSet::of(
        &[
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest,
        ],
    ),
    ..CardDef::BLANK
};

/// "Whenever this permanent enters or attacks, create a tapped colorless land token named Everywhere that is every basic land type." (CR 608.2, 111.1).
fn create_tapped_colorless_land_token(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut tokens_0 = Vec::new();
    let def_0_0 = g.card_id(&EVERYWHERE);
    tokens_0.extend(std::iter::repeat((def_0_0, item.controller)).take(1));
    let words_0 = g.words(item.object);
    g.create_tokens_worded(&tokens_0, true, None, words_0)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        impending::REMOVE_A_TIME_COUNTER,
        triggered(
            when_asked(
                EventMask(EventMask::ZONE_CHANGE.0 | EventMask::ATTACKS.0),
                when_permanent_enters_attacks,
            ),
            create_tapped_colorless_land_token,
            "Whenever this permanent enters or attacks, create a tapped colorless land token named Everywhere that is every basic land type.",
        ),
    ],
    statics: &[impending::NOT_A_CREATURE],
    enters: Some(impending::enters::<4>),
    ways: &[impending::way(mana("{1}{G}{G}"), "Impending 4—{1}{G}{G}")],
    ..oracle::OVERLORD_OF_THE_HAUNTWOODS
};

pub static EXTRA: &[&CardDef] = &[&EVERYWHERE];
