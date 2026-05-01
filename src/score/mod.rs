mod torent;

pub struct ScoreHandler{
    entitys: Vec<ScoreEntyti>,
}

pub trait Scorable {

}


struct ScoreEntyti<T: Scorable>(T);
