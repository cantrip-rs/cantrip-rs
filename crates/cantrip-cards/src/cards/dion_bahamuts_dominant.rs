use cantrip_api::*;
use cantrip_mechanics::statics::{have_flying, itself_during_your_turn, other_of_subtype_you_control};
use cantrip_mechanics::{effects, exile_and_return, exile_and_return_transformed, lands, targets, tokens};
use cantrip_oracle as oracle;

/// "Other Knights you control" (CR 613.1f), "during your turn" (CR 102.1): `other_of_subtype_you_control` for Knights, only
/// while the source's controller is the active player. Knight is read through the words of Dion's text (CR 612).
fn other_knights_during_your_turn(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    q.state().turn.active == q.state().obj(source).controller
        && other_of_subtype_you_control::<{ lands::sub(Subtype::Knight) }>(q, source, affected, so_far)
}

/// "When Dion enters, create a 2/2 white Knight creature token." (CR 111.4)
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::KNIGHT_2_2_WHITE, 1)?;
    Ok(())
}

/// "{4}{W}{W}, {T}: Exile Dion, then return it to the battlefield transformed under its owner's control." (CR 712.14a):
/// nothing happens if it has left the battlefield already (CR 400.7).
fn transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

/// I, II \u{2014} Wings of Light: "Put a +1/+1 counter on each other creature you control. Those creatures gain flying until end
/// of turn." (CR 608.2c, 611.2a): the creatures its controller controls as it resolves but the Saga, all at once.
fn wings_of_light(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others: Vec<ObjectId> =
        g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).is(CardType::Creature)).collect();
    effects::counters_on_each(g, &others, CounterKind::PlusOne, 1);
    g.end_sentence();
    for id in others {
        effects::gain_until_end_of_turn(g, item.object, id, Keyword::Flying);
    }
    Ok(())
}

/// III \u{2014} Gigaflare: "Destroy target permanent. Exile Bahamut, then return it to the battlefield (front face up)." (CR 701.8,
/// 712.14a): with its target gone the ability does not resolve at all (CR 608.2b), and Bahamut is sacrificed as a Saga whose last
/// chapter has left the stack (CR 714.4); destroyed himself, he is no longer there to exile (CR 400.7).
fn gigaflare(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_first_target(g, item)?;
    g.end_sentence();
    if g.is_on_battlefield(item.object) {
        exile_and_return(g, &[item.object])?;
    }
    Ok(())
}

/// Bahamut, Warden of Light, the back face: a white 5/5 Saga Dragon creature with flying (CR 714).
pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, wings_of_light, "I, II \u{2014} Wings of Light \u{2014} Put a +1/+1 counter on each other creature you control. Those creatures gain flying until end of turn."),
        chapter(2, wings_of_light, "I, II \u{2014} Wings of Light \u{2014} Put a +1/+1 counter on each other creature you control. Those creatures gain flying until end of turn."),
        chapter(3, gigaflare, "III \u{2014} Gigaflare \u{2014} Destroy target permanent. Exile Bahamut, then return it to the battlefield (front face up).")
            .targets(&[targets::PERMANENT]),
    ],
    ..oracle::BAHAMUT_WARDEN_OF_LIGHT
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::THIS_ENTERS, enters, "When Dion enters, create a 2/2 white Knight creature token."),
        ability(cost("{4}{W}{W}").and(TAP), transform, "{4}{W}{W}, {T}: Exile Dion, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.")
            .timing(Timing::Sorcery),
    ],
    statics: &[
        static_ability(Layer::Ability, itself_during_your_turn, have_flying, "Dragonfire Dive \u{2014} During your turn, Dion and other Knights you control have flying."),
        static_ability(Layer::Ability, other_knights_during_your_turn, have_flying, "Dragonfire Dive \u{2014} During your turn, Dion and other Knights you control have flying."),
    ],
    back: Some(&BACK),
    ..oracle::DION_BAHAMUTS_DOMINANT
};

pub static EXTRA: &[&CardDef] = &[&tokens::KNIGHT_2_2_WHITE];
