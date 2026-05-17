use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use hex;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use rocket::http::{ContentType, Status};
use rocket::request::Outcome;
use rocket::response::content::RawJson;
use rocket::serde::json::Json;
use rocket::{get, post, State};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::sync::RwLock;

use crate::db::Store;
use crate::secure;

// ----- constants -----

const CAPTCHA_DIR:        &str = "/home/blucher/development/HentaiTraker/captcha_images";
const CONFIG_PATH:        &str = "/home/blucher/development/HentaiTraker/captcha_images/captcha.toml";
const CHALLENGE_TTL_SECS: u64  = 300;
const TOKEN_TTL_SECS:     u64  = 120;
const GATE_TTL_SECS:      u64  = 3600;

// ----- config types -----

#[derive(Deserialize)]
struct ConfigEntry {
    folder:  String,
    correct: Vec<usize>,
}

// ----- runtime types -----

struct CaptchaEntry {
    prompt:  String,
    images:  Vec<PathBuf>,
    correct: Vec<usize>,
}

struct Slot {
    path:      PathBuf,
    is_target: bool,
}

struct Challenge {
    slots:     Vec<Slot>,
    issued_at: Instant,
}

// ----- store -----

pub struct CaptchaStore {
    entries:   Vec<CaptchaEntry>,
    challenges: RwLock<HashMap<String, Challenge>>,
    tokens:     RwLock<HashMap<String, Instant>>,
}

impl CaptchaStore {
    pub async fn load() -> Arc<Self> {
        let entries = Self::load_config().await;
        info!("captcha: загружено {} заданий", entries.len());
        Arc::new(Self {
            entries,
            challenges: RwLock::new(HashMap::new()),
                 tokens:     RwLock::new(HashMap::new()),
        })
    }

    async fn load_config() -> Vec<CaptchaEntry> {
        let raw = match fs::read_to_string(CONFIG_PATH).await {
            Ok(s)  => s,
            Err(e) => { error!("captcha: не могу прочитать {CONFIG_PATH}: {e}"); return vec![]; }
        };

        let config: HashMap<String, ConfigEntry> = match toml::from_str(&raw) {
            Ok(c)  => c,
            Err(e) => { error!("captcha: ошибка парсинга TOML: {e}"); return vec![]; }
        };

        let mut entries = Vec::new();

        for (prompt, cfg) in config {
            let dir = format!("{CAPTCHA_DIR}/{}", cfg.folder);
            let mut images: Vec<PathBuf> = Vec::new();

            match fs::read_dir(&dir).await {
                Err(e) => { error!("captcha: не могу открыть папку '{dir}': {e}"); continue; }
                Ok(mut rd) => {
                    while let Ok(Some(entry)) = rd.next_entry().await {
                        if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                            images.push(entry.path());
                        }
                    }
                }
            }

            images.sort();

            let max_idx = images.len().saturating_sub(1);
            let valid   = cfg.correct.iter().all(|&i| i <= max_idx);
            if !valid {
                error!("captcha: задание '{prompt}' — индекс correct выходит за пределы ({} файлов)", images.len());
                continue;
            }
            if cfg.correct.is_empty() {
                error!("captcha: задание '{prompt}' — correct пустой");
                continue;
            }

            info!("captcha: задание '{prompt}' — {} картинок, {} правильных", images.len(), cfg.correct.len());
            entries.push(CaptchaEntry { prompt, images, correct: cfg.correct });
        }

        entries
    }

    pub async fn create_challenge(&self) -> Option<(String, String, Vec<String>)> {
        if self.entries.is_empty() { return None; }

        // ----- выбор задания (rng дропается до await) -----
        let (prompt, slots) = {
            let mut rng   = rand::rng();
            let entry     = self.entries.choose(&mut rng)?;
            let prompt    = entry.prompt.clone();

            let mut slots: Vec<Slot> = entry.images.iter().enumerate()
            .map(|(i, path)| Slot {
                path:      path.clone(),
                 is_target: entry.correct.contains(&i),
            })
            .collect();

            slots.shuffle(&mut rng);
            (prompt, slots)
        };

        let id   = secure::get_token(24);
        let urls: Vec<String> = (0..slots.len())
        .map(|i| format!("/api/captcha/image/{id}/{i}"))
        .collect();

        let challenge = Challenge { slots, issued_at: Instant::now() };

        let mut map = self.challenges.write().await;
        map.retain(|_, c| c.issued_at.elapsed().as_secs() < CHALLENGE_TTL_SECS);
        map.insert(id.clone(), challenge);

        Some((id, prompt, urls))
    }

    pub async fn get_image(&self, challenge_id: &str, idx: usize) -> Option<PathBuf> {
        let map = self.challenges.read().await;
        let ch  = map.get(challenge_id)?;
        if ch.issued_at.elapsed().as_secs() >= CHALLENGE_TTL_SECS { return None; }
        ch.slots.get(idx).map(|s| s.path.clone())
    }

    pub async fn verify(&self, challenge_id: &str, selected: &[usize]) -> bool {
        let mut map = self.challenges.write().await;

        let ch = match map.remove(challenge_id) {
            Some(c) => c,
            None    => {
                warn!("captcha::verify — challenge не найден: id='{}'", challenge_id);
                return false;
            }
        };

        if ch.issued_at.elapsed().as_secs() >= CHALLENGE_TTL_SECS {
            warn!("captcha::verify — challenge истёк: elapsed={}s", ch.issued_at.elapsed().as_secs());
            return false;
        }

        let mut correct: Vec<usize> = ch.slots.iter().enumerate()
        .filter(|(_, s)| s.is_target)
        .map(|(i, _)| i)
        .collect();
        correct.sort_unstable();

        let mut given = selected.to_vec();
        given.sort_unstable();

        if given != correct {
            warn!("captcha::verify — неверный ответ: given={:?} correct={:?}", given, correct);
            return false;
        }

        true
    }

    pub async fn verify_and_token(&self, challenge_id: &str, selected: &[usize]) -> Option<String> {
        if !self.verify(challenge_id, selected).await { return None; }
        let token = secure::get_token(32);
        let mut tokens = self.tokens.write().await;
        tokens.retain(|_, issued| issued.elapsed().as_secs() < TOKEN_TTL_SECS);
        tokens.insert(token.clone(), Instant::now());
        Some(token)
    }

    pub async fn consume_token(&self, token: &str) -> bool {
        let mut map = self.tokens.write().await;
        match map.remove(token) {
            Some(issued) if issued.elapsed().as_secs() < TOKEN_TTL_SECS => true,
            _ => false,
        }
    }
}

// ----- fingerprint request guard -----

pub struct Fingerprint(String);

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Fingerprint {
    type Error = ();

    async fn from_request(req: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = req.client_ip().map(|ip| ip.to_string()).unwrap_or_default();
        let ua = req.headers().get_one("User-Agent").unwrap_or("");
        let mut h = Sha256::new();
        h.update(format!("{ip}|{ua})"));
        Outcome::Success(Fingerprint(hex::encode(h.finalize())))
    }
}

// ----- request body -----

#[derive(Deserialize)]
pub struct VerifyBody {
    challenge_id: String,
    selected:     Vec<usize>,
}

// ----- GET /api/captcha/challenge -----

#[get("/challenge")]
pub async fn challenge(store: &State<Arc<CaptchaStore>>) -> (Status, RawJson<String>) {
    match store.create_challenge().await {
        Some((id, prompt, urls)) => (
            Status::Ok,
            RawJson(json!({ "challenge_id": id, "target": prompt, "images": urls }).to_string()),
        ),
        None => (
            Status::ServiceUnavailable,
            RawJson(json!({ "error": "no_challenges" }).to_string()),
        ),
    }
}

// ----- POST /api/captcha/verify -----

#[post("/verify", data = "<body>", format = "json")]
pub async fn verify(
    body:  Json<VerifyBody>,
    store: &State<Arc<CaptchaStore>>,
) -> (Status, RawJson<String>) {
    match store.verify_and_token(&body.challenge_id, &body.selected).await {
        Some(token) => (
            Status::Ok,
            RawJson(json!({ "captcha_token": token }).to_string()),
        ),
        None => (
            Status::BadRequest,
            RawJson(json!({ "error": "wrong_answer" }).to_string()),
        ),
    }
}

// ----- GET /api/captcha/image/<challenge_id>/<idx> -----

#[get("/image/<challenge_id>/<idx>")]
pub async fn image(
    challenge_id: String,
    idx:          usize,
    store:        &State<Arc<CaptchaStore>>,
) -> Option<(ContentType, Vec<u8>)> {
    let path = store.get_image(&challenge_id, idx).await?;
    let data = fs::read(&path).await.ok()?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let ct  = match ext {
        "jpg" | "jpeg" => ContentType::JPEG,
        "png"          => ContentType::PNG,
        "webp"         => ContentType::new("image", "webp"),
        "gif"          => ContentType::GIF,
        _              => ContentType::Binary,
    };

    Some((ct, data))
}

// ----- GET /api/captcha/gate -----

#[get("/gate")]
pub async fn gate_check(
    fp:    Fingerprint,
    store: &State<Arc<Store>>,
) -> RawJson<String> {
    let key    = format!("captcha_gate:{}", fp.0);
    let passed = store.captcha_gate_check(&key).await;
    RawJson(json!({ "passed": passed }).to_string())
}

// ----- POST /api/captcha/gate -----

#[post("/gate", data = "<body>", format = "json")]
pub async fn gate_verify(
    fp:            Fingerprint,
    body:          Json<VerifyBody>,
    store:         &State<Arc<Store>>,
    captcha_store: &State<Arc<CaptchaStore>>,
) -> (Status, RawJson<String>) {
    if captcha_store.verify(&body.challenge_id, &body.selected).await {
        let key = format!("captcha_gate:{}", fp.0);
        store.captcha_gate_set(&key, GATE_TTL_SECS).await;
        (Status::Ok, RawJson(json!({ "ok": true }).to_string()))
    } else {
        (
            Status::BadRequest,
         RawJson(json!({ "error": "wrong_answer" }).to_string()),
        )
    }
}
