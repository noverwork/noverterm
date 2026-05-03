use lettre::message::{header::ContentType, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[derive(Clone)]
pub struct PasswordResetEmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from: String,
}

#[derive(Clone)]
pub struct PasswordResetMailer {
    config: PasswordResetEmailConfig,
}

impl PasswordResetMailer {
    pub fn new(config: PasswordResetEmailConfig) -> Self {
        Self { config }
    }

    pub async fn send_reset_link(&self, to: &str, reset_link: &str) -> Result<(), String> {
        let email = Message::builder()
            .from(
                self.config
                    .from
                    .parse()
                    .map_err(|error| format!("invalid SMTP_FROM address: {error}"))?,
            )
            .to(to
                .parse()
                .map_err(|error| format!("invalid reset recipient address: {error}"))?)
            .subject("Reset your Noverterm password")
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(password_reset_text_body(reset_link)),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(password_reset_html_body(reset_link)),
                    ),
            )
            .map_err(|error| format!("failed to build password reset email: {error}"))?;

        let credentials = Credentials::new(
            self.config.smtp_username.clone(),
            self.config.smtp_password.clone(),
        );
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.smtp_host)
            .map_err(|error| format!("failed to configure SMTP relay: {error}"))?
            .port(self.config.smtp_port)
            .credentials(credentials)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|error| format!("failed to send password reset email: {error}"))?;

        Ok(())
    }
}

fn password_reset_text_body(reset_link: &str) -> String {
    format!(
        "Reset your Noverterm password\n\nWe received a request to reset your Noverterm password. This link expires in 1 hour.\n\nReset password:\n{reset_link}\n\nIf you did not request this, you can ignore this email."
    )
}

fn password_reset_html_body(reset_link: &str) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Reset your Noverterm password</title>
  </head>
  <body style="margin:0;padding:0;background:#070b12;color:#e5edf8;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Arial,sans-serif;">
    <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background:#070b12;padding:32px 16px;">
      <tr>
        <td align="center">
          <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="max-width:560px;border:1px solid rgba(148,163,184,0.18);border-radius:28px;background:#0b1220;box-shadow:0 24px 64px rgba(0,0,0,0.42);overflow:hidden;">
            <tr>
              <td style="padding:28px 28px 0 28px;">
                <div style="display:inline-block;border:1px solid rgba(34,211,238,0.22);border-radius:999px;background:rgba(34,211,238,0.08);color:#a5f3fc;font-size:11px;font-weight:700;letter-spacing:0.18em;text-transform:uppercase;padding:8px 12px;">Noverterm</div>
                <h1 style="margin:22px 0 10px 0;color:#f8fafc;font-size:28px;line-height:1.2;font-weight:700;">Reset your password</h1>
                <p style="margin:0;color:#94a3b8;font-size:15px;line-height:1.7;">We received a request to reset your Noverterm password. Use the secure link below to set a new one.</p>
              </td>
            </tr>
            <tr>
              <td style="padding:28px;">
                <a href="{reset_link}" style="display:inline-block;border-radius:16px;background:#67e8f9;color:#06111a;text-decoration:none;font-size:15px;font-weight:800;padding:14px 22px;box-shadow:0 12px 32px rgba(34,211,238,0.22);">Reset password</a>
                <p style="margin:24px 0 0 0;color:#cbd5e1;font-size:13px;line-height:1.7;">This link expires in <strong style="color:#f8fafc;">1 hour</strong>. If you did not request a password reset, you can safely ignore this email.</p>
              </td>
            </tr>
            <tr>
              <td style="padding:0 28px 28px 28px;">
                <div style="border-radius:18px;border:1px solid rgba(148,163,184,0.14);background:rgba(15,23,42,0.72);padding:16px;">
                  <p style="margin:0 0 8px 0;color:#64748b;font-size:12px;line-height:1.6;">If the button does not work, copy and paste this URL into your browser:</p>
                  <p style="margin:0;word-break:break-all;color:#a5f3fc;font-size:12px;line-height:1.6;">{reset_link}</p>
                </div>
              </td>
            </tr>
          </table>
          <p style="max-width:560px;margin:18px auto 0 auto;color:#475569;font-size:12px;line-height:1.6;">Noverterm security notification</p>
        </td>
      </tr>
    </table>
  </body>
</html>"##
    )
}
