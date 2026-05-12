use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
};

// ----- config -----

pub struct EmailConfig {
    pub smtp_host:     String,
    pub smtp_port:     u16,
    pub smtp_user:     String,
    pub smtp_password: String,
    pub from_address:  String,
}

impl EmailConfig {
    pub fn from_env() -> Self {
        Self {
            smtp_host:     std::env::var("SMTP_HOST").expect("SMTP_HOST not set"),
            smtp_port:     std::env::var("SMTP_PORT").unwrap_or_else(|_| "587".to_string())
            .parse().expect("SMTP_PORT must be a number"),
            smtp_user:     std::env::var("SMTP_USER").expect("SMTP_USER not set"),
            smtp_password: std::env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD not set"),
            from_address:  std::env::var("SMTP_FROM").expect("SMTP_FROM not set"),
        }
    }
}

// ----- error -----

#[derive(Debug)]
pub enum EmailError {
    Build(lettre::error::Error),
    Send(lettre::transport::smtp::Error),
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::Build(e) => write!(f, "email build error: {e}"),
            EmailError::Send(e)  => write!(f, "email send error: {e}"),
        }
    }
}

impl std::error::Error for EmailError {}

// ----- send -----

pub async fn send_email(
    config: &EmailConfig,
    to:     &str,
    subject: &str,
    body:   &str,
) -> Result<(), EmailError> {
    let message = Message::builder()
    .from(config.from_address.parse().unwrap())
    .to(to.parse().unwrap())
    .subject(subject)
    .header(ContentType::TEXT_PLAIN)
    .body(body.to_string())
    .map_err(EmailError::Build)?;

    let creds = Credentials::new(
        config.smtp_user.clone(),
        config.smtp_password.clone(),
    );

    let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
    .unwrap()
    .port(config.smtp_port)
    .credentials(creds)
    .build();

    transport.send(message).await.map_err(EmailError::Send)?;

    Ok(())
}
