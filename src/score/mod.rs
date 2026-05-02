use crate::score::torent::TorentState;

mod torent;

pub struct ScoreHandler{
    entitys: Vec<ScoreEntyti>,
}

pub enum ScoreEfect {
    Increment,
    Decriment,
    FullDecriment,
    Nothing,
    Shtraff   // бля каккто подебильному штраф на англиском будет поэтому транслит
}

pub enum NextEvent {
    Torent(TorentState)
}

pub trait Scorable {
    fn get_score(self) -> u32;
    fn next(self, event: NextEvent) -> (ScoreEfect, u32);
}



struct ScoreEntyti<T: Scorable>(T);

impl ScoreHandler {
    fn from_row_db_str
}
