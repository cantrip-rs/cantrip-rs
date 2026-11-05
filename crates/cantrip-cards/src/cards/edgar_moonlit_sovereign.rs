use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;
/// The intervening if asks all casts in the turn, even before Edgar entered (CR 603.4).
fn no_casts(q:&dyn Query,me:&Asker)->bool { !q.history().casts_by(me.controller).any(|_| true) }
fn grow(g:&mut dyn Game,i:&StackItem)->Result<(),Suspend>{g.add_counters(i.object,CounterKind::PlusOne,2);Ok(())}
/// The recipients are creatures with a counter as the ability resolves (CR 608.2h).
fn team(g:&mut dyn Game,i:&StackItem)->Result<(),Suspend>{
 let ids:Vec<_>=g.permanents_of(i.controller).into_iter().filter(|&id|g.chars(id).is(CardType::Creature)&&g.obj(id).counters(CounterKind::PlusOne)>0).collect();
 let puts:Vec<(ObjectId,CounterKind,i64)>=ids.into_iter().map(|id|(id,CounterKind::PlusOne,1)).collect();
 g.add_counters_together(&puts);Ok(())
}
pub static CARD:CardDef=CardDef{
 abilities:&[triggered(triggers::YOUR_END_STEP,grow,"At the beginning of your end step, if you didn't cast a spell this turn, put two +1/+1 counters on Edgar.").condition(no_casts),ability(cost("{4}{G}"),team,"{4}{G}: Put a +1/+1 counter on each creature you control with a +1/+1 counter on it.")],..oracle::EDGAR_MOONLIT_SOVEREIGN
};
