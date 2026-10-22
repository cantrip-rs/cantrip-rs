use cantrip_api::*;
use cantrip_mechanics::{choose, effects, tokens, triggers};
use cantrip_oracle as oracle;

const GNOME_SOLDIER_TEXT: &str = "This creature's power and toughness are each equal to the number of artifacts and/or creatures you control.";

/// "This creature's power and toughness are each equal to the number of artifacts and/or creatures you control" (CR 604.3,
/// layer 7a): each permanent its controller controls that is an artifact, a creature or both counts once, the token itself
/// among them, as the earlier layers left them.
fn artifacts_and_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let you = state.obj(source).controller;
    let n = state
        .battlefield()
        .iter()
        .filter(|&&id| state.obj(id).controller == you && {
            let c = q.chars(id);
            c.is(CardType::Artifact) || c.is(CardType::Creature)
        })
        .count() as i16;
    chars.power = Some(n);
    chars.toughness = Some(n);
}

/// The white Gnome Soldier artifact creature token both faces make, with "This creature's power and toughness are each equal
/// to the number of artifacts and/or creatures you control."
pub static GNOME_SOLDIER: CardDef = CardDef {
    name: "Gnome Soldier Token",
    token_name: Some(&[Subtype::Gnome, Subtype::Soldier]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Gnome, Subtype::Soldier]),
    statics: &[characteristic_defining(Layer::PtDefine, artifacts_and_creatures_you_control, GNOME_SOLDIER_TEXT).lost_pt(0, 0)],
    text: GNOME_SOLDIER_TEXT,
    ..CardDef::BLANK
};

/// "Create a white Gnome Soldier artifact creature token with ..." (CR 111.1).
fn gnome_soldier(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &GNOME_SOLDIER, 1)?;
    Ok(())
}

const TAP_FIVE: Question = Question::new(
    "Transform",
    "tap five untapped artifacts and/or creatures you control to transform it?",
    "You choose the five next; they stay tapped until they untap",
    "Tap five",
    "Don't transform",
);

/// "You may tap five untapped artifacts and/or creatures you control. If you do, transform Thousand Moons Smithy" (CR 701.26a,
/// 701.27): a player can't choose to do what they can't (CR 609.3), so with fewer than five nothing is asked; the five are
/// chosen as it resolves, the Smithy itself among them if it is untapped. The transform is the ability's own instruction about
/// its source (CR 701.27f), which does nothing to a Smithy that has left the battlefield (CR 400.7).
fn may_tap_five_to_transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let untapped: Vec<ObjectId> = g
        .permanents_of(you)
        .into_iter()
        .filter(|&id| !g.obj(id).tapped && {
            let c = g.chars(id);
            c.is(CardType::Artifact) || c.is(CardType::Creature)
        })
        .collect();
    if untapped.len() < 5 || !effects::ask_may(g, you, item.object, Purpose::Ask(&TAP_FIVE))? {
        return Ok(());
    }
    for permanent in choose::cards(g, you, item.object, Purpose::Tap, untapped, 5, false)? {
        g.tap(permanent);
    }
    g.transform_by(item.object, item)?;
    Ok(())
}

/// "Whenever you cast an artifact or creature spell using mana produced by Barracks of the Thousand" (CR 601.2i, 603.2, 106.3):
/// a spell its controller cast, an artifact or a creature as it is on the stack, some of whose total cost was paid with mana
/// this permanent produced (the history's `ManaSpent` records of the cast) - however little, and with whatever else.
fn artifact_or_creature_spell_cast_with_its_mana(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::SpellCast { spell, player, .. }
            if player == me.controller
                && (q.chars(spell).is(CardType::Artifact) || q.chars(spell).is(CardType::Creature))
                && q.history().cast_using_mana_from(spell, me.source) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// "Mana produced by Barracks of the Thousand" is told apart from other mana (CR 106.3): its controller chooses whether to
/// spend it.
fn its_own_mana(_: &dyn Query, me: &Asker, source: ObjectId) -> bool {
    source == me.source
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::W), "{T}: Add {W}."),
        triggered(
            when_asked(EventMask::SPELL_CAST, artifact_or_creature_spell_cast_with_its_mana),
            gnome_soldier,
            "Whenever you cast an artifact or creature spell using mana produced by Barracks of the Thousand, create a white Gnome Soldier artifact creature token with \"This creature's power and toughness are each equal to the number of artifacts and/or creatures you control.\"",
        ),
    ],
    mana_told_apart: &[its_own_mana],
    ..oracle::BARRACKS_OF_THE_THOUSAND
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            gnome_soldier,
            "When Thousand Moons Smithy enters, create a white Gnome Soldier artifact creature token with \"This creature's power and toughness are each equal to the number of artifacts and/or creatures you control.\"",
        ),
        triggered(
            triggers::YOUR_FIRST_MAIN_PHASE,
            may_tap_five_to_transform,
            "At the beginning of your first main phase, you may tap five untapped artifacts and/or creatures you control. If you do, transform Thousand Moons Smithy.",
        ),
    ],
    back: Some(&BACK),
    ..oracle::THOUSAND_MOONS_SMITHY
};

pub static EXTRA: &[&CardDef] = &[&GNOME_SOLDIER];
