use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::config::Config;

pub struct EmailService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl EmailService {
    pub fn new(config: &Config) -> Result<Self, String> {
        let smtp_host = config
            .smtp_host
            .as_deref()
            .ok_or("SMTP_HOST not configured")?;
        let smtp_username = config
            .smtp_username
            .as_deref()
            .ok_or("SMTP_USERNAME not configured")?;
        let smtp_password = config
            .smtp_password
            .as_deref()
            .ok_or("SMTP_PASSWORD not configured")?;
        let smtp_from = config
            .smtp_from
            .as_deref()
            .ok_or("SMTP_FROM not configured")?;

        let creds = Credentials::new(smtp_username.to_string(), smtp_password.to_string());

        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host)
            .map_err(|e| format!("SMTP setup failed: {e}"))?
            .port(config.smtp_port)
            .credentials(creds)
            .build();

        Ok(EmailService {
            transport,
            from: smtp_from.to_string(),
        })
    }

    pub async fn send_invite(
        &self,
        to_email: &str,
        trip_name: &str,
        inviter_name: &str,
        invite_url: &str,
    ) -> Result<(), String> {
        let body = format!(
            "{inviter_name} invited you to join the trip \"{trip_name}\" on Rice.\n\n\
             Click here to join: {invite_url}\n\n\
             This invite expires in 7 days."
        );

        let email = Message::builder()
            .from(
                self.from
                    .parse()
                    .map_err(|e| format!("Invalid from address: {e}"))?,
            )
            .to(to_email
                .parse()
                .map_err(|e| format!("Invalid to address: {e}"))?)
            .subject(format!("You're invited to join \"{trip_name}\" on Rice"))
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .map_err(|e| format!("Failed to build email: {e}"))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| format!("Failed to send email: {e}"))?;

        Ok(())
    }
}
