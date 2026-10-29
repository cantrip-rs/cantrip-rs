use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

fn power_per_officer_token(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let you = state.obj(source).controller;
    let officer = q.words(source).subtype(Subtype::Officer);
    let n = state.battlefield().iter().filter(|&&id| state.obj(id).controller == you
        && q.chars(id).has_subtype(officer) && state.obj(id).token)
        .count() as i16;
    chars.power = chars.power.map(|v| v + n);
}

fn officer_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::OFFICER, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, cantrip_mechanics::statics::itself, power_per_officer_token, "This creature gets +1/+0 for each Officer token you control.")],
    abilities: &[triggered(triggers::NONCREATURE_SPELL_YOU_CAST, officer_token, "Whenever you cast a noncreature spell, create a 1/1 red Officer creature token.")],
    ..oracle::ACADEMY_INSTRUCTOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::OFFICER];
